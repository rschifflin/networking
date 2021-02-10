use std::collections::hash_map::OccupiedEntry;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use mio::{Poll, Token};
use mio::net::UdpSocket as MioUdpSocket;

use crate::types::FromDaemon as ToService;
use crate::state::{State, FSM};

pub fn handle(mut entry: OccupiedEntry<Token, (State, MioUdpSocket)>, poll: &Poll) {
  let (ref mut state, ref mut socket) = entry.get_mut();
  let (ref buf_read, ref _buf_write, ref read_cond, ref status) = *state.shared;

  if status.load(Ordering::SeqCst) != 0 {
    poll.registry().deregister(socket).expect("Could not deregister");
    entry.remove();
    return;
  }

  let mut buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");
  match socket.recv(&mut state.buf_local) {
    Ok(size) => {
      //NOTE: The consensus _seems_ to be that we should notify the condvar while still holding the lock
      buf.push_back(&state.buf_local[..size]).map(|_| read_cond.notify_one());
      drop(buf);

      // If we were listening, we now know we have a live connection to accept
      if let FSM::Listen{ tx } = &state.fsm {
        tx.send(
          ToService::Connection(Arc::clone(&state.shared))
        ).expect("Could not finish listening with connection state");
        state.fsm = FSM::Connected;
      }
    },

    Err(e) => {
      drop(buf);
      if e.kind() == std::io::ErrorKind::WouldBlock {} // This is fine for mio
      else {} // Handle bad errors here!
    }
  }
}
