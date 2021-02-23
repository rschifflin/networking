use std::collections::hash_map::OccupiedEntry;

use mio::{Poll, Token};
use log::warn;

use crate::socket::{Socket, PeerType};
use crate::daemon::poll;

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;

pub fn handle(mut token_entry: TokenEntry, poll: &Poll) {
  let socket = token_entry.get_mut();
  match socket.peer_type {
    // Since Direct sockets aren't listeners, this should never occur
    PeerType::Direct(_, _) => warn!("Attempted to stop listening on non-listen socket {:?}", socket.io),

    // All listeners are Passive sockets
    PeerType::Passive { ref mut listen, ref peers, .. } => {
      *listen = None;

      // We can free the resource if there are no peers and we aren't listening
      if peers.len() == 0 {
        poll::deregister_io(poll, &mut socket.io);
        token_entry.remove();
      }
    }
  }
}
