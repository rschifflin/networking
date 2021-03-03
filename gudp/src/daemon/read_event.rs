use std::collections::hash_map::OccupiedEntry;

use log::trace;
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
      // WouldBlock is fine for mio, we just try again later
      if e.kind() != std::io::ErrorKind::WouldBlock {
        // SOMEDAY: Convey more error info to app side. Maybe set remote drop flags based on errorkind?
        let errno = e.raw_os_error();

        // Iterate thru all peers, signalling io error then removing them
        match socket.peer_type {
          PeerType::Direct(_, ref mut state) => state.on_io_error(errno, s),
          PeerType::Passive { ref mut peers, .. } => {
            for (_addr, peer_state) in peers.iter() {
              peer_state.on_io_error(errno, s);
            }
          },
        };

        trace!("OnReadable: IO encountered error, dropping all peers.");
        poll::deregister_io(&mut socket.io, s);
        token_entry.remove();
      }
    },

    Ok((size, peer_addr)) => {
      match socket.peer_type {
        PeerType::Passive { ref mut peers, ref listen, .. } => {
          match (peers.get_mut(&peer_addr), listen) {
            /* Socket noise */
            (None, None) => { },

            /* Existing peer */
            (Some(state), _) => {
              // Returns FALSE if the socket can be cleaned up (read from app end is closed and write to peer buffer is empty)
              // Returns TRUE otherwise
              if !state.read(socket.local_addr, peer_addr, size, s) {
                trace!("OnReadable: Peer is finished, dropping {}", peer_addr);
                peers.remove(&peer_addr);
                // If no peers left and not actively listening, close and free the resource
                if peers.len() == 0 && listen.is_none() {
                  trace!("OnReadbale: All peers are finished, dropping IO");
                  poll::deregister_io(&mut socket.io, s);
                  token_entry.remove();
                }
              };
            },

            /* create+handle new peer */
            (None, Some(conn_opts)) => {
              let socket_id = (token, peer_addr);
              let mut peer_state = State::init(socket_id, conn_opts.clone(), s);

              // If state update fails, we simply don't insert the new peer
              if peer_state.read(socket.local_addr, peer_addr, size, s) {
                peers.insert(peer_addr, peer_state);
              };
            },
          }
        }

        PeerType::Direct(peer_addr, ref mut state) => {
          if !state.read(socket.local_addr, peer_addr, size, s) {
            poll::deregister_io(&mut socket.io, s);
            token_entry.remove();
          };
        }
      }
    }
  }
}
