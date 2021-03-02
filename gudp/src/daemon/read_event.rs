use std::collections::hash_map::OccupiedEntry;
use std::time::Instant;
use mio::Token;

use crate::socket::{Socket, PeerType};
use crate::state::State;
use crate::daemon::{LoopLocalState, poll};

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;
pub fn handle(mut token_entry: TokenEntry, s: &mut LoopLocalState) {
  let token = *token_entry.key();
  let socket = token_entry.get_mut();

  // TODO: Read in loop until we hit WOULDBLOCK
  match socket.io.recv_from(&mut s.buf_local) {
    Err(e) => {
      if e.kind() == std::io::ErrorKind::WouldBlock {} // This is fine for mio, try again later
      else {
        // TODO: Handle errors explicitly. Set remote drop flags based on errorkind
        // Add error flags we can set when we have a semantic error that has no underlying errno code.
        let errno = e.raw_os_error();

        // Iterate thru all peers, removing them
        match socket.peer_type {
          PeerType::Direct(_, ref mut state) => {
            let (ref buf_read, ref _buf_write, ref status) = *state.shared;
            let buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");
            status.set_io_err(errno);
            buf.notify_all();
            drop(buf);
          },

          PeerType::Passive { ref mut peers, .. } => {
            for (_addr, peer_state) in peers.iter() {
              let (ref buf_read, ref _buf_write, ref status) = *peer_state.shared;
              let buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");
              status.set_io_err(errno);
              buf.notify_all();
              drop(buf);
            }
          },
        };
        poll::deregister_io(&mut socket.io, s);
        token_entry.remove();
      }
    },

    Ok((size, peer_addr)) => {
      let when = Instant::now();
      match socket.peer_type {
        PeerType::Passive { ref mut peers, ref listen, .. } => {
          match (peers.get_mut(&peer_addr), listen) {
            /* Handle existing peer */
            (Some(state), _) => {
              if !state.read(socket.local_addr, peer_addr, size, when, s) {
                peers.remove(&peer_addr); // Remove closed connection
                // No peers left and not actively listening. Close and free the resource
                if peers.len() == 0 && listen.is_none() {
                  poll::deregister_io(&mut socket.io, s);
                  token_entry.remove();
                }
              };
            },

            /* create+handle new peer */
            (None, Some(conn_opts)) => {
              let socket_id = (token, peer_addr);
              let mut peer_state = State::init(when, socket_id, conn_opts.clone(), s);

              // If state update fails, we simply don't insert the new peer
              if peer_state.read(socket.local_addr, peer_addr, size, when, s) {
                peers.insert(peer_addr, peer_state);
              };
            },

            (None, None) => return, // Discard unrecognized peer msgs when not listening
          }
        }

        PeerType::Direct(peer_addr, ref mut state) => {
          if !state.read(socket.local_addr, peer_addr, size, when, s) {
            poll::deregister_io(&mut socket.io, s);
            token_entry.remove();
          };
        }
      }
    }
  }
}
