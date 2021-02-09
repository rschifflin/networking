use std::net::UdpSocket;
use std::sync::{Arc, Condvar};

use crossbeam::channel;
use mio::Waker;

use crate::Service;
use crate::types::SharedRingBuf;
use crate::types::{FromDaemon, ToDaemon};

// A user-facing GUDP Connection interface
pub struct Connection {
    waker: Arc<Waker>,
    buf_read: SharedRingBuf,
    buf_write: SharedRingBuf,
    read_cond: Arc<Condvar>
}

impl Connection {
    pub fn new(waker: Arc<Waker>, buf_read: SharedRingBuf, buf_write: SharedRingBuf, read_cond: Arc<Condvar>) -> Connection {
      Connection { waker, buf_read, buf_write, read_cond }
    }

    pub fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
      let mut buf_write = self.buf_write.lock().expect("Could not acquire unpoisoned write lock");
      match buf_write.push_back(buf) {
        Some(size) => {
          drop(buf_write);
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
      let mut buf_read = self.buf_read.lock().expect("Could not acquire unpoisoned read lock");
      while buf_read.count() <= 0 {
        buf_read = self.read_cond.wait(buf_read).expect("Could not wait on condvar");
      }

      return buf_read.pop_front(buf).map(std::io::Result::Ok).unwrap_or_else(|| {
        std::io::Result::Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "Nothing to recv"))
      });
    }

    pub fn try_recv(&self, buf: &mut [u8]) -> Option<std::io::Result<usize>> {
      let mut buf_read = self.buf_read.lock().expect("Could not acquire unpoisoned read lock");
      if buf_read.count() > 0 {
        return buf_read.pop_front(buf).map(std::io::Result::Ok).or_else(|| {
          Some(
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
  if let Ok(FromDaemon::Connection(buf_read, buf_write, read_cond)) = rx.recv() {
    return Some(Connection::new(waker, buf_read, buf_write, read_cond));
  }
  None
}
