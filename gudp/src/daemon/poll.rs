use std::io;
use std::collections::HashMap;
use std::net::UdpSocket as StdUdpSocket;

use log::warn;
use mio::{Poll, Token, Interest};
use mio::net::UdpSocket as MioUdpSocket;

use bring::Bring;
use cond_mutex::CondMutexGuard;

use crate::state::State;
use crate::types::READ_BUFFER_TAG;

// Only call when you've ensured status.is_closed() is true!
// Otherwise notified readers might sleep again.
// See notes in read_event and write_event
pub fn close_remote_socket<'a>(
  poll: &'a Poll,
  socket: &'a mut MioUdpSocket,
  cond_lock: CondMutexGuard<Bring, READ_BUFFER_TAG>
) {
  cond_lock.notify_all();
  drop(cond_lock);
  deregister_io(poll, socket);
}

pub fn handle_failure(e: io::Error, states: &mut HashMap<Token, (State, MioUdpSocket)>) -> io::Error {
  // Call to the system selector failed.
  // We cannot perform any evented IO without it.
  // It's possible this error has non-fatal variants, but it's
  // likely platform-specific. For now we treat them all as fatal.
  let errno = e.raw_os_error();
  for (_, (state, _socket)) in states.into_iter() {
    let (ref buf_read, ref _buf_write, ref status) = *state.shared;
    let buf_read = buf_read.lock().expect("Could not acquire unpoisoned read lock");
    status.set_io_err(errno);
    buf_read.notify_all();
  }

  e
}

pub fn register_io(poll: &Poll, io: StdUdpSocket, next_conn_id: &mut usize) -> Option<(Token, MioUdpSocket)> {
  // Create a mio wrapper for the socket.
  let mut conn = MioUdpSocket::from_std(io);

  // Associate this io with a token
  let token = Token(*next_conn_id);
  *next_conn_id += 1;

  // Register this io with its token for polling
  poll.registry()
    .register(&mut conn, token, Interest::READABLE | Interest::WRITABLE)
    .map(|_| (token, conn))
    .ok()
}

// If the deregister fails, we'll silently leak the file descriptor
// For now, simply log if this occurs.
// TODO: We could bubble up hanging resources to the main loop,
// where we iterate on trying to deregister them.
pub fn deregister_io(poll: &Poll, io: &mut MioUdpSocket) {
  poll.registry().deregister(io).unwrap_or_else(|e| {
    warn!("Unable to deregister socket from poll on close. The socket fd may leak! Reason: {}", e);
  });
}
