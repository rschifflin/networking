use std::net::UdpSocket;
use std::sync::Arc;

use crossbeam::channel;
use mio::Waker;

use crate::Service;
use crate::types::SharedConnState;
use crate::types::{FromDaemon, ToDaemon};

// A user-facing GUDP Connection interface
pub struct Connection {
    waker: Arc<Waker>,
    shared: Arc<SharedConnState>
}

impl Connection {
    pub fn new(waker: Arc<Waker>, shared: Arc<SharedConnState>) -> Connection {
      Connection { waker, shared }
    }

    pub fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
      let (ref _buf_read, ref buf_write, ref _read_cond) = *self.shared;
      let mut buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");
      let push_result = buf_write.push_back(buf);
      drop(buf_write);
      match push_result {
        Some(size) => {
          self.waker.wake().expect("Could not wake"); // Wake on send to flush all writes immediately
          Ok(size)
        },
        None => {
          std::io::Result::Err(
            std::io::Error::new(std::io::ErrorKind::WouldBlock, "Nothing to send")
          )
        }
      }
    }

    pub fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
      let (ref buf_read, ref _buf_write, ref read_cond) = *self.shared;
      let mut buf_read = buf_read.lock().expect("Could not acquire unpoisoned read lock");
      while buf_read.count() <= 0 {
        buf_read = read_cond.wait(buf_read).expect("Could not wait on condvar");
      }
      let pop_result = buf_read.pop_front(buf);
      drop(buf_read);
      // TODO: This error might be that the buffer is just too small! Should we truncate?
      return pop_result.map(std::io::Result::Ok).unwrap_or_else(|| {
        std::io::Result::Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "Nothing to recv"))
      });
    }

    pub fn try_recv(&self, buf: &mut [u8]) -> Option<std::io::Result<usize>> {
      let (ref buf_read, ref _buf_write, ref _read_cond) = *self.shared;
      let mut buf_read = buf_read.lock().expect("Could not acquire unpoisoned read lock");
      if buf_read.count() > 0 {
        let pop_result = buf_read.pop_front(buf);
        drop(buf_read);
        return pop_result.map(std::io::Result::Ok).or_else(|| {
          Some(
            // TODO: This error might be that the buffer is just too small! Should we truncate?
            std::io::Result::Err(
              std::io::Error::new(std::io::ErrorKind::WouldBlock, "Nothing to recv")
            )
          )
        });
      }
      None
    }
}

pub fn connect(service: &Service, socket: UdpSocket) -> Option<Connection> {
  let (tx, rx) = channel::bounded(1);
  let (tx_to_daemon, waker) = service.clone_parts();
  tx_to_daemon.send(ToDaemon::Connect(socket, tx))
    .expect("Could not send new connection to daemon");

  waker.wake() // Force daemon to handle this new connection immediately
    .expect("Could not wake daemon to receive new connection");

  // Block until connection is established or the daemon dies trying I guess
  // TODO: Result<Connection> in case any other msg or the daemon thread dying
  if let Ok(FromDaemon::Connection(shared)) = rx.recv() {
    return Some(Connection::new(waker, shared));
  }
  None
}
