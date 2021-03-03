use std::collections::hash_map::OccupiedEntry;
use std::net::SocketAddr;

use log::trace;
use mio::Token;

use crate::socket::{Socket, PeerType};
use crate::daemon::{LoopLocalState, poll};

// Handling app writes are subtly different than socket writeable events
// In the case of a direct connection, the two are identical
// In the case of a passive listener connection...
//  App writes are only for a given peer, and add to the pending writers list on block.
//  Writeable events walk the list and try to write for all pending writers of an io until the io would block again.
type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;
pub fn handle(mut token_entry: TokenEntry, peer_addr: SocketAddr, s: &mut LoopLocalState) {
  let socket = token_entry.get_mut();
  match &mut socket.peer_type {
    PeerType::Passive { peers, ref listen, pending_writes } => {
      match (peers.get_mut(&peer_addr), listen) {
        (None, _) => { /* discard socket noise */ },
        (Some(peer_state), _) => {
          match peer_state.write(&mut socket.io, peer_addr, s) {
            // Success, pending write fulfilled if present
            Ok(true) => { pending_writes.remove(&peer_addr); },
            // Peer hung up and no reads left, can clean up the resource
            Ok(false) => {
              trace!("App Write: Peer is finished, dropping {}", peer_addr);
              peers.remove(&peer_addr);

              if peers.len() == 0 && listen.is_none() {
                trace!("App Write: All peers are finished, dropping IO");
                poll::deregister_io(&mut socket.io, s);
                token_entry.remove();
              }
            },
            Err(e) => {
              // WouldBlock is fine for mio, we just try again later
              if e.kind() == std::io::ErrorKind::WouldBlock {
                // mark pending write if absent
                pending_writes.insert(peer_addr);
              } else {
                let errno = e.raw_os_error();
                for (_addr, peer_state) in peers.iter() {
                  peer_state.on_io_error(errno, s);
                }
                  trace!("App Write: IO encountered error, dropping all peers. Caused by {}", peer_addr);
                poll::deregister_io(&mut socket.io, s);
                token_entry.remove();
              }
            }
          }
        },
      }
    },

    PeerType::Direct(addr, state) => {
      match state.write(&mut socket.io, *addr, s) {
        Ok(true) => { /* Success */ },
        Ok(false) => { // Resource ready to be cleaned up
          poll::deregister_io(&mut socket.io, s);
          token_entry.remove();
        },
        Err(e) => {
          if e.kind() != std::io::ErrorKind::WouldBlock {
            let errno = e.raw_os_error();
            state.on_io_error(errno, s);
            poll::deregister_io(&mut socket.io, s);
            token_entry.remove();
          }
        }
      }
    }
  }
}
