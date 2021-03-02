use std::collections::hash_map::OccupiedEntry;
use std::net::SocketAddr;
use std::time::Instant;
use mio::{Poll, Token};

use crate::socket::{Socket, PeerType};
use crate::daemon::poll;
use crate::types::Expired;
use crate::timer::{Timers, TimerKind};

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;

pub fn handle<'a, T>(mut token_entry: TokenEntry, peer_addr: SocketAddr, kind: TimerKind, poll: &Poll, timers: &'a mut T)
  where T: Timers<'a,
    Item = ((Token, SocketAddr), TimerKind),
    Expired = Expired<'a, T>> {

  let when = Instant::now();
  let socket = token_entry.get_mut();
  match socket.peer_type {
    PeerType::Direct(_, ref mut state) => {
      if !state.timer(kind, when, timers) {
        poll::deregister_io(poll, &mut socket.io);
        token_entry.remove();
      }
    },

    PeerType::Passive { ref mut peers, ref listen, .. } => {
      if let Some(state) = peers.get_mut(&peer_addr) {
        if !state.timer(kind, when, timers) {
          peers.remove(&peer_addr);
          if peers.len() == 0 && listen.is_none() {
            poll::deregister_io(poll, &mut socket.io);
            token_entry.remove();
          }
        }
      }
    }
  }
}