use std::collections::hash_map::OccupiedEntry;
use std::net::SocketAddr;
use std::time::Instant;

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
  let when = Instant::now();
  match &mut socket.peer_type {
    PeerType::Passive { peers, listen, pending_writes } => {
      match (peers.get_mut(&peer_addr), listen) {
        (Some(peer_state), _) => {
          match peer_state.write(&mut socket.io, peer_addr, when, s) {
            Ok(true) => { pending_writes.remove(&peer_addr); },
            Ok(false) => { pending_writes.insert(peer_addr); },
            Err(e) => { // Deregister io
              let errno = e.raw_os_error();

              // peer_state.write already signals this conn's io error
              // remove it to get the list of just all sibling connections that must die
              peers.remove(&peer_addr);

              for (_addr, peer_state) in peers.iter() {
                let (ref buf_read, ref _buf_write, ref status) = *peer_state.shared;
                let buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");
                status.set_io_err(errno);
                buf.notify_all();
                drop(buf);
              }

              poll::deregister_io(&mut socket.io, s);
              token_entry.remove();
              return;
            }
          }
        },
        (None, _) => { /* discard socket noise */ },
      }
    },
    PeerType::Direct(addr, state) => {
      match state.write(&mut socket.io, *addr, when, s) {
        // If we receive wouldblock that's ok, since this peer is 1:1 with the underlying io
        // and will be chosen to write when the io becomes writable
        Ok(_) => (),
        Err(_) => {
          // Deregister io
          poll::deregister_io(&mut socket.io, s);
          token_entry.remove();
        }
      }
    }
  }
}
