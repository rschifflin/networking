use std::collections::hash_map::OccupiedEntry;

use log::trace;
use mio::Token;

use clock::Clock;

use crate::socket::{Socket, PeerType};
use crate::state::State;
use crate::daemon::{self, poll};
use crate::constants::header;

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;
pub fn handle<C: Clock>(mut token_entry: TokenEntry, s: &mut daemon::State<C>) {
  let token = *token_entry.key();
  let socket = token_entry.get_mut();
  let local_addr = socket.local_addr;

  // socket read loop
  // early return on wouldblock
  // break on cases where we're finished with the io, to perform io cleanup at the end
  loop {
    match socket.io.recv_from(&mut s.buf_local) {
      Err(e) => {
        // WouldBlock is fine for mio, we just try again later
        if e.kind() == std::io::ErrorKind::WouldBlock {
          return;
        } else {
          // SOMEDAY: Convey more error info to app side. Maybe set remote drop flags based on errorkind?
          let errno = e.raw_os_error();

          // Iterate thru all peers, signalling io error then removing them
          match socket.peer_type {
            PeerType::Direct(_, ref mut state) => state.on_io_error(errno),
            PeerType::Passive { ref mut peers, .. } => {
              for (_addr, peer_state) in peers.iter() {
                peer_state.on_io_error(errno);
              }
            },
          };

          trace!("OnReadable: IO encountered error, dropping all peers.");
          break; // Breaking the loop without wouldblock indicates a failure state
        }
      },

      Ok((size, peer_addr)) => {
        // Filter out non-conforming protocol bits as socket noise
        if size < header::SIZE_BYTES { continue; }
        if s.buf_local[..4] != header::MAGIC_BYTES { continue; }

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
                    trace!("OnReadable: All peers are finished, dropping IO");
                    break;
                  }
                };
              },

              /* create+handle new peer */
              (None, Some(conn_opts)) => {
                let socket_id = (token, peer_addr);
                trace!("Creating new peer: {}", peer_addr);
                let mut peer_state = State::init(local_addr, socket_id, conn_opts.clone(), s);

                // If state update fails, we simply don't insert the new peer
                if peer_state.read(socket.local_addr, peer_addr, size, s) {
                  trace!("> Inserting new peer: {}", peer_addr);
                  peers.insert(peer_addr, peer_state);
                } else {
                  trace!("> Ignoring new peer: {}", peer_addr);
                };
              },
            }
          }

          PeerType::Direct(peer_addr, ref mut state) => {
            if !state.read(socket.local_addr, peer_addr, size, s) { break };
          }
        }
      }
    }
  }

  // Reach here when the state machine is terminal or a fatal io error occurs
  poll::deregister_io(&mut socket.io, s);
  token_entry.remove();
}
