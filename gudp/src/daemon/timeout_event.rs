use std::collections::hash_map::OccupiedEntry;
use std::net::SocketAddr;
use mio::{Poll, Token};

use crate::socket::{Socket, PeerType};
use crate::daemon::poll;

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;

pub fn handle(mut token_entry: TokenEntry, peer_addr: SocketAddr, poll: &Poll) {
  let socket = token_entry.get_mut();
  match socket.peer_type {
    PeerType::Direct(_, ref mut state) => {
      let (ref buf_read, ref _buf_write, ref status) = *state.shared;
      let buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");
      status.set_io_hup();
      buf.notify_all();
      drop(buf);

      poll::deregister_io(poll, &mut socket.io);
      token_entry.remove();
    },

    PeerType::Passive { ref mut peers, ref listen, .. } => {
      if let Some(peer_state) = peers.get(&peer_addr) {
        let (ref buf_read, ref _buf_write, ref status) = *peer_state.shared;
        let buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");
        status.set_io_hup();
        buf.notify_all();
        drop(buf);

        peers.remove(&peer_addr);
        if peers.len() == 0 && listen.is_none() {
          poll::deregister_io(poll, &mut socket.io);
          token_entry.remove();
        }
      }
    }
  }
}
