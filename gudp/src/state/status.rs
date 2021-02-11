/// Status is a shared state atomic usize with semantic bitflags to indicate
/// statuses about a connection.
/// Importantly, it respects the following invariants:
/// Connections may be either Open or Closed.
/// Writes that change the connection from Open->Open, Open->Closed or Closed->Closed are always allowed.
/// Writes that change the connection from Closed->Open are disallowed.
/// Therefore two clients may race to close a connection, but once a Closed connection is observed, no future writes will ever bring it back to Open.

use std::sync::atomic::{AtomicUsize, Ordering};

const FLAG_LOCAL_DROP: usize = 1usize.rotate_right(1);
const FLAG_REMOTE_DROP: usize = 1usize.rotate_right(2);
const FLAG_CLOSED: usize = FLAG_LOCAL_DROP | FLAG_REMOTE_DROP;

#[derive(Debug)]
pub struct Status {
  inner: AtomicUsize
}

impl Status {
  pub fn new(inner: AtomicUsize) -> Status {
    Status { inner }
  }

  // Indicate the user has dropped the connection
  pub fn set_local_drop(&self) {
    // Set the local drop flag and preserve the rest
    self.inner.fetch_or(FLAG_LOCAL_DROP, Ordering::SeqCst);
  }

  // Indicate the socket has dropped the connection
  pub fn set_remote_drop(&self) {
    // Set the remote drop flag and preserve the rest
    self.inner.fetch_or(FLAG_REMOTE_DROP, Ordering::SeqCst);
  }

  // Will check various flags to determine if the connection qualifes as closed
  // NOTE: Once connections are considered closed, they will never unclose.
  // So if is_closed() == true, you know no races will occur to unclose the conn.
  pub fn is_closed(&self) -> bool {
    (self.inner.load(Ordering::SeqCst) & FLAG_CLOSED) != 0
  }

  pub fn is_open(&self) -> bool {
    !self.is_closed()
  }
}
