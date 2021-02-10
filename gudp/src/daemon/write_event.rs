use std::collections::hash_map::OccupiedEntry;
use std::sync::atomic::Ordering;

use mio::{Poll, Token};
use mio::net::UdpSocket as MioUdpSocket;

use bring::WithOpt;

use crate::state::State;

pub fn handle(mut entry: OccupiedEntry<Token, (State, MioUdpSocket)>, poll: &Poll) {
  let (ref mut state, ref mut socket) = entry.get_mut();
  let (ref _buf_read, ref buf_write, ref _read_cond, ref status) = *state.shared;

  if status.load(Ordering::SeqCst) != 0 {
    poll.registry().deregister(socket).expect("Could not deregister");
    entry.remove();
    return;
  }

  let mut buf = buf_write.lock().expect("Could not acquire unpoisoned write lock");

  let buf = &mut *buf;
  let buf_local = &mut state.buf_local;
  match buf.with_front(buf_local, |buf_local, bytes| {
    let send = socket.send(&buf_local[..bytes]);
    let opt = match send {
      Ok(_) => WithOpt::Pop,
      Err(_) => WithOpt::Peek
    };
    (send, opt)
  }) {
    None => (), // Nothing was on the ring or our buf was too small. Simply no-op the write
    Some(Ok(_wrote)) => (), // There was data on the buffer and we were able to pop it and send it!
    Some(Err(e)) => {
      if e.kind() == std::io::ErrorKind::WouldBlock {} // There was data on the buffer but we would've blocked if we tried to send it, so we left it alone
      else {} // There was data on the buffer but the socket errored when we tried to send it! We should close the resource here
    }
  }
}
