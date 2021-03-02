use std::collections::hash_map::OccupiedEntry;
use std::net::SocketAddr;

use mio::Token;

use crate::socket::{Socket, PeerType};
use crate::daemon::{LoopLocalState, poll};

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;
pub fn handle(mut token_entry: TokenEntry, pending_write_keybuf: &mut Vec<SocketAddr>, s: &mut LoopLocalState) {
  let socket = token_entry.get_mut();
  match &mut socket.peer_type {
    PeerType::Passive { ref mut peers, ref listen, ref mut pending_writes } => {
      pending_write_keybuf.extend(pending_writes.iter().copied());
      for peer_addr in pending_write_keybuf.iter() {
        match (peers.get_mut(peer_addr), listen) {
          (Some(peer_state), _) => {
            match peer_state.write(&mut socket.io, *peer_addr, s) {
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

                poll::deregister_io(&mut socket.io, s);
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
      match state.write(&mut socket.io, *addr, s) {
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
