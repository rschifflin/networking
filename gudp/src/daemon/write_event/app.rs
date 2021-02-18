use std::collections::hash_map::OccupiedEntry;
use std::net::SocketAddr;

use mio::{Poll, Token};

use crate::socket::{Socket, PeerType};
use crate::daemon::poll;

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;
pub fn handle(mut token_entry: TokenEntry, peer_addr: SocketAddr, buf_local: &mut [u8], poll: &Poll) {
  let socket = token_entry.get_mut();
  match &mut socket.peer_type {
    PeerType::Passive { peers, listen, pending_writes } => {
      match (peers.get_mut(&peer_addr), listen) {
        (Some(peer_state), _) => {
          match peer_state.write(&mut socket.io, peer_addr, buf_local) {
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

              poll::deregister_io(poll, &mut socket.io);
              token_entry.remove();
              return;
            }
          }
        },
        (None, _) => { /* discard socket noise */ },
      }
    },
    PeerType::Direct(addr, state) => {
      match state.write(&mut socket.io, *addr, buf_local) {
        // If we receive wouldblock that's ok, since this peer is 1:1 with the underlying io
        // and will be chosen to write when the io becomes writable
        Ok(_) => (),
        Err(_) => {
          // Deregister io
          poll::deregister_io(poll, &mut socket.io);
          token_entry.remove();
        }
      }
    }
  }
}
