use std::collections::hash_map::OccupiedEntry;
use std::net::SocketAddr;

use log::trace;
use mio::Token;

use clock::Clock;

use crate::socket::{Socket, PeerType};
use crate::daemon::{self, poll};
use crate::timer::TimerKind;

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;

pub fn handle<C: Clock>(mut token_entry: TokenEntry, peer_addr: SocketAddr, kind: TimerKind, s: &mut daemon::State<C>) {
  let socket = token_entry.get_mut();
  match socket.peer_type {
    PeerType::Direct(_, ref mut state) => {
      if !state.timer(kind, s) {
        poll::deregister_io(&mut socket.io, s);
        token_entry.remove();
      }
    },

    PeerType::Passive { ref mut peers, ref listen, .. } => {
      if let Some(state) = peers.get_mut(&peer_addr) {
        if !state.timer(kind, s) {
          trace!("OnTimeout: Peer is finished, dropping {}", peer_addr);

          peers.remove(&peer_addr);
          if peers.len() == 0 && listen.is_none() {
            trace!("OnTimeout: All peers are finished, dropping IO");
            poll::deregister_io(&mut socket.io, s);
            token_entry.remove();
          }
        }
      }
    }
  }
}
