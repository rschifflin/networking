use std::collections::hash_map::OccupiedEntry;
use std::sync::Arc;
use std::io;
use mio::{Poll, Token};
use log::warn;

use crate::socket::{Socket, PeerType};
use crate::types::FromDaemon as ToService;
use crate::state::{State, FSM, Closer};
use crate::daemon::poll;
use crate::error;

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;

pub fn handle(mut token_entry: TokenEntry, buf_local: &mut [u8], poll: &Poll) {
  let mut socket = token_entry.get_mut();

  // TODO: Read in loop until we hit WOULDBLOCK
  match socket.io.recv_from(buf_local) {
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

          PeerType::Passive { ref mut peers, ref mut listen } => {
            for (_addr, peer_state) in peers.iter() {
              let (ref buf_read, ref _buf_write, ref status) = *peer_state.shared;
              let buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");
              status.set_io_err(errno);
              buf.notify_all();
              drop(buf);
            }
          },
        };
        poll::deregister_io(poll, &mut socket.io);
        token_entry.remove();
      }
    },

    Ok((size, peer_addr)) => {
      match socket.peer_type {
        PeerType::Passive { ref mut peers, ref listen } => {
          match (peers.get_mut(&peer_addr), listen) {
            /* Handle existing peer */
            (Some(mut state), _) => {
              if let Err(closer) = state.read(peer_addr, buf_local, size) {
                peers.remove(&peer_addr); // Remove closed connection
                if let Closer::IO = closer {
                  warn!("IO error closed a connection, but only hup was expected! Other connections on this IO may linger");
                }

                // No peers left and not actively listening. Close and free the resource
                if peers.len() == 0 && listen.is_none() {
                  poll::deregister_io(poll, &mut socket.io);
                  token_entry.remove();
                }
              };
            },

            /* create+handle new peer */
            (None, Some(listen_opts)) => {
              let mut peer_state = State::init_connect(
                listen_opts.token,
                listen_opts.tx_to_service.clone(),
                listen_opts.tx_on_write.clone(),
                Arc::clone(&listen_opts.waker));

              // If it fails, we simply don't insert the new peer
              peer_state.read(peer_addr, buf_local, size).map(|_| {
                peers.insert(peer_addr, peer_state);
              });
            },

            (None, None) => return, // Discard socket noise
          }
        }

        PeerType::Direct(addr, ref mut state) => {
          if peer_addr != addr { return; } // Discard irrelevant socket noise */
          if let Err(_) = state.read(addr, buf_local, size) {
            poll::deregister_io(poll, &mut socket.io);
            token_entry.remove();
          };
        }
      }
    }
  }
}
