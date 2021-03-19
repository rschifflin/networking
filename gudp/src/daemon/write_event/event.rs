use std::collections::hash_map::OccupiedEntry;
use std::net::SocketAddr;

use log::trace;
use mio::Token;

use clock::Clock;

use crate::socket::{Socket, PeerType};
use crate::daemon::{self, poll};

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;
pub fn handle<C: Clock>(mut token_entry: TokenEntry, pending_write_keybuf: &mut Vec<SocketAddr>, s: &mut daemon::State<C>) {
  let socket = token_entry.get_mut();
  match &mut socket.peer_type {
    PeerType::Passive { ref mut peers, ref listen, ref mut pending_writes } => {
      pending_write_keybuf.extend(pending_writes.iter().copied());
      for peer_addr in pending_write_keybuf.iter() {
        match (peers.get_mut(peer_addr), listen) {
          (None, _) => { /* discard socket noise */ },
          (Some(peer_state), _) => {
            match peer_state.write(&mut socket.io, *peer_addr, s) {
              // Success; pending write fulfilled
              Ok(true) => { pending_writes.remove(peer_addr); },
              // Peer hung up and no reads left, can clean up the resource
              Ok(false) => {
                trace!("OnWriteable: Peer is finished, dropping {}", peer_addr);
                peers.remove(&peer_addr);

                if peers.len() == 0 && listen.is_none() {
                  trace!("OnWriteable: All peers are finished, dropping IO");
                  poll::deregister_io(&mut socket.io, s);
                  token_entry.remove();
                  break; // Stop iterating peers, they're all gone
                }
              },
              Err(e) => {
                // WouldBlock is fine for mio, we just try again later
                if e.kind() == std::io::ErrorKind::WouldBlock {
                  break; // Stop iterating peers, the io would block
                }

                // SOMEDAY: Convey more error info to app side. Maybe set remote drop flags based on errorkind?
                let errno = e.raw_os_error();
                for (_addr, peer_state) in peers.iter() {
                  peer_state.on_io_error(errno);
                }

                trace!("OnWriteable: IO encountered error, dropping all peers. Caused by {}", peer_addr);
                poll::deregister_io(&mut socket.io, s);
                token_entry.remove();
                break; // Stop iterating peers, they're all dead
              }
            }
          },
        }
      }
    },

    PeerType::Direct(addr, state) => {
      match state.write(&mut socket.io, *addr, s) {
        Ok(_) => { /* Success */ },
        Err(e) => {
          if e.kind() != std::io::ErrorKind::WouldBlock {
            // SOMEDAY: Convey more error info to app side. Maybe set remote drop flags based on errorkind?
            let errno = e.raw_os_error();
            state.on_io_error(errno);
            poll::deregister_io(&mut socket.io, s);
            token_entry.remove();
          }
        }
      }
    }
  }
}
