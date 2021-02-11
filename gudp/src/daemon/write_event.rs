use std::collections::hash_map::OccupiedEntry;

use mio::{Poll, Token};
use mio::net::UdpSocket as MioUdpSocket;

use bring::WithOpt;

use crate::state::State;

pub fn handle(mut entry: OccupiedEntry<Token, (State, MioUdpSocket)>, _poll: &Poll) {
  let (ref mut state, ref mut socket) = entry.get_mut();
  let (ref _buf_read, ref buf_write, ref _status) = *state.shared;

  // NOTE: Unlike the READ case, WRITEs never sleep on a condvar. If a write would overflow the write buffer, we
  // return an Err::WriteZero immediately instead.
  // One open question is SHOULD we add a block-on-write interface? (Leaning towards yes for completeness)
  // Doing so would add another condvar and require us to acquire BOTH the read+write locks before deregistering,
  // signalling both condvars.

  // TODO: Acquire read lock and check if the socket is closed to perform a deregister.
  // TODO: Loop thru and write the entire buffer until a WOULDBLOCK or some socket error
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
