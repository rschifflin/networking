use std::collections::hash_map::OccupiedEntry;
use std::net::SocketAddr;
use std::time::Instant;

use mio::{Poll, Token};

use crate::socket::{self, Socket, PeerType};
use crate::daemon::poll;
use crate::types::Expired;
use crate::timer::{Timers, TimerKind};

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;
pub fn handle<'a, T>(mut token_entry: TokenEntry, pending_write_keybuf: &mut Vec<SocketAddr>, buf_local: &mut [u8], poll: &Poll, timers: &'a mut T)
where T: Timers<'a, Item = (socket::Id, TimerKind), Expired = Expired<'a, T>> {
  let socket = token_entry.get_mut();
  let when = Instant::now();
  match &mut socket.peer_type {
    PeerType::Passive { ref mut peers, ref listen, ref mut pending_writes } => {
      pending_write_keybuf.extend(pending_writes.iter().copied());
      for peer_addr in pending_write_keybuf.iter() {
        match (peers.get_mut(peer_addr), listen) {
          (Some(peer_state), _) => {
            match peer_state.write(&mut socket.io, *peer_addr, buf_local, when, timers) {
              // Success and still no blocking
              Ok(true) => { pending_writes.remove(peer_addr); },

              // WouldBlock; stop for now awaiting next writeable event
              Ok(false) => { return; },

              // Underlying IO failed. Stop and deregister
              Err(e) => {
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
      }
    },

    PeerType::Direct(addr, state) => {
      match state.write(&mut socket.io, *addr, buf_local, when, timers) {
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
