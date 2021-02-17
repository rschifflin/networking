use std::collections::hash_map::OccupiedEntry;
use std::net::SocketAddr;

use mio::{Poll, Token};

use crate::state::State;
use crate::socket::{Socket, PeerType};
use crate::daemon::poll;

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;
pub fn handle(mut token_entry: TokenEntry, peer_addr: SocketAddr, buf_local: &mut [u8], poll: &Poll) {
  let socket = token_entry.get_mut();
  match &mut socket.peer_type {
    PeerType::Passive { peers, listen } => {
      match (peers.get_mut(&peer_addr), listen) {
        (Some(mut state), _) => {
          // TODO: Propagate errors
          state.write(&mut socket.io, peer_addr, buf_local);
        },
        (None, _) => { /* discard socket noise */ },
      }
    },

    PeerType::Direct(addr, state) => {
      // TODO: Do we care if addr != peer_addr here?
      // TODO: Propagate errors
      state.write(&mut socket.io, *addr, buf_local);
    }
  }
}
