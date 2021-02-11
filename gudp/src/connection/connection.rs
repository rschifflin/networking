use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crossbeam::channel;
use mio::Waker;

use crate::Service;
use crate::types::SharedConnState;
use crate::types::{FromDaemon, ToDaemon};
use crate::error;

use std::io;

// A user-facing GUDP Connection interface
pub struct Connection {
    waker: Arc<Waker>,
    shared: Arc<SharedConnState>
}

impl Drop for Connection {
  fn drop(&mut self) {
    let (_, _, _, ref status) = *self.shared;
    status.store(1, Ordering::SeqCst);
  }
}

impl Connection {
    pub fn new(waker: Arc<Waker>, shared: Arc<SharedConnState>) -> Connection {
      Connection { waker, shared }
    }

    pub fn send(&self, buf: &[u8]) -> io::Result<usize> {
      let (ref _buf_read, ref buf_write, ref _read_cond, ref status) = *self.shared;
      if status.load(Ordering::SeqCst) != 0 { return Err(error::send_on_closed()); }

      let mut buf_write = buf_write.lock().map_err(error::poisoned_write_lock)?;
      let push_result = buf_write.push_back(buf);
      drop(buf_write);
      match push_result {
        Some(size) => {
          self.waker.wake().map_err(error::wake_failed)?; // Wake on send to flush all writes immediately
          Ok(size)
        },
        None => { Err(error::no_space_to_write()) }
      }
    }

    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
      let (ref buf_read, ref _buf_write, ref read_cond, ref status) = *self.shared;
      if status.load(Ordering::SeqCst) != 0 { return Err(error::recv_on_closed()); }

      let mut buf_read = buf_read.lock().map_err(error::poisoned_read_lock)?;
      while buf_read.count() <= 0 {
        buf_read = read_cond.wait(buf_read).map_err(error::poisoned_read_lock)?
      }
      let pop_result = buf_read.pop_front(buf);

      // Finished all contentious reading; signal the next reader if needed then drop the lock
      if buf_read.count() > 0 { read_cond.notify_one(); }
      drop(buf_read);

      // NOTE: Pop result is only None when there are no reads (not happening here)
      //       or the buffer to copy to is just too small! Should we truncate?
      pop_result.map(Ok).unwrap_or_else(|| Err(error::no_space_to_read()))
    }

    pub fn try_recv(&self, buf: &mut [u8]) -> Option<io::Result<usize>> {
      let (ref buf_read, ref _buf_write, ref _read_cond, ref status) = *self.shared;
      if status.load(Ordering::SeqCst) != 0 { return Some(Err(error::recv_on_closed())); }

      buf_read.lock().map_err(error::poisoned_read_lock).and_then(|mut buf_read| {
        if buf_read.count() > 0 {
          let pop_result = buf_read.pop_front(buf);
          drop(buf_read);
          match pop_result {
            Some(size) => Ok(Some(size)),
            None => Err(error::no_space_to_read())
          }
        } else {
          Ok(None)
        }
      }).transpose()
    }
}

pub fn connect(service: &Service, socket: UdpSocket) -> io::Result<Connection> {
  let (tx, rx) = channel::bounded(1);
  let (tx_to_daemon, waker) = service.clone_parts();
  tx_to_daemon.send(ToDaemon::Connect(socket, tx))
    .map_err(error::cannot_send_to_daemon)?;

  // Force daemon to handle this new connection immediately
  waker.wake().map_err(error::wake_failed)?;

  match rx.recv() {
    Ok(FromDaemon::Connection(shared)) => Ok(Connection::new(waker, shared)),
    Err(e) => Err(error::cannot_recv_from_daemon(e))
  }
}
