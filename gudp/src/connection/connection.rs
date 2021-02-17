use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use crossbeam::channel;
use log::warn;

use crate::Service;
use crate::Listener;
use crate::types::{SharedConnState, OnWrite, FromDaemon, ToDaemon};
use crate::error;

use std::io;

// A user-facing GUDP Connection interface
pub struct Connection {
  on_write: Box<OnWrite>,
  shared: Arc<SharedConnState>,
  id: (SocketAddr, SocketAddr)
}

impl Drop for Connection {
  fn drop(&mut self) {
    let (_, _, ref status) = *self.shared;
    status.set_client_hup();
  }
}

impl Connection {
    pub fn new(on_write: Box<OnWrite>, shared: Arc<SharedConnState>, id: (SocketAddr, SocketAddr)) -> Connection {
      Connection { on_write, shared, id }
    }

    pub fn send(&self, buf: &[u8]) -> io::Result<usize> {
      let (ref _buf_read, ref buf_write, ref status) = *self.shared;
      status.check_client()?;

      let mut buf_write = buf_write.lock().map_err(error::poisoned_write_lock)?;
      let push_result = buf_write.push_back(buf);
      drop(buf_write);
      match push_result {
        Some(size) => (self.on_write)(size), // Wake on send to flush all writes immediately
        None => Err(error::no_space_to_write())
      }
    }

    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
      let (ref buf_read, ref _buf_write, ref status) = *self.shared;
      let mut buf_read = buf_read.lock().map_err(error::poisoned_read_lock)?;

      let mut health = status.check_client();
      while buf_read.count() <= 0 && health.is_ok() {
        buf_read = buf_read.wait().map_err(error::poisoned_read_lock)?;
        health = status.check_client();
      }

      // We arrive here only if the read buffer has data or the status is closed.
      // If the read buffer doesn't have data, it means the status is closed.
      // Nothing left to do but report an error here (and on all future reads).
      // NOTE: UNLIKE the case where buf_read has data, the daemon calls notify_all() when a conn is closed.
      // This means every thread will wake up, observe the conn is closed, break its loop and arrive here.
      // Noticeably, they will NEVER sleep on the condvar and NEVER need to be signalled again. So we don't need to notify_one() here.
      if buf_read.count() <= 0 {
        health.and_then(|_| Err(error::unknown()))?;
      }

      // We arrive here only if the read buffer has data. We don't care about the connection state until the
      // read buffer has been drained.
      let pop_result = buf_read.pop_front(buf);

      // Finished all contentious reading; signal the next reader if needed then drop the lock
      if buf_read.count() > 0 { buf_read.notify_one(); }
      drop(buf_read);

      // NOTE: Pop result is only None when there are no reads (not happening here)
      //       or the buffer to copy to is just too small!
      //       Thus we signal UnexpectedEOF to indicate there was no space to read.
      //       The connection is still OK- the data is still waiting to be read if we bring a bigger buffer
      pop_result.map(Ok).unwrap_or_else(|| Err(error::no_space_to_read()))
    }


    // Much simpler case since its nonblocking nature means we never worry about the condvar
    pub fn try_recv(&self, buf: &mut [u8]) -> Option<io::Result<usize>> {
      let (ref buf_read, ref _buf_write, ref status) = *self.shared;
      buf_read.lock().map_err(error::poisoned_read_lock).and_then(|mut buf_read| {
        if buf_read.count() > 0 {
          let pop_result = buf_read.pop_front(buf);
          drop(buf_read);
          match pop_result {
            Some(size) => Ok(Some(size)),
            None => Err(error::no_space_to_read())
          }
        } else {
          status.check_client().map(|_| None)
        }
      }).transpose()
    }

    pub fn local_addr(&self) -> SocketAddr {
      self.id.0
    }

    pub fn peer_addr(&self) -> SocketAddr {
      self.id.1
    }
}

pub fn connect<A: ToSocketAddrs>(service: &Service, socket: UdpSocket, to_addr: A) -> io::Result<Connection> {
  let peer_addr = to_addr.to_socket_addrs().and_then(|mut addr| {
    addr.next()
      .map(Ok)
      .unwrap_or_else(|| Err(error::socket_addr_failed_to_resolve()))
  })?;

  let (tx, rx) = channel::bounded(2);
  let (tx_to_daemon, waker) = service.clone_parts();
  tx_to_daemon.send(ToDaemon::Connect(socket, tx, peer_addr))
    .map_err(error::cannot_send_to_daemon)?;

  // Force daemon to handle this new connection immediately
  waker.wake().map_err(error::wake_failed)?;

  // Close any spurious listeners
  rx.recv()
    .map_err(error::cannot_recv_from_daemon)
    .and_then(|received| match received {
      FromDaemon::Connection(on_write, shared, id) => Ok(Connection::new(on_write, shared, id)),

      // This is unexpected. We only wanted a Connection message.
      // Close the given listener and signal the issue;
      FromDaemon::Listener => {
        warn!("When trying to register directly connected socket, received Listener instead");
        let listener = Listener { rx };
        drop(listener);
        Err(error::unexpected_recv_from_daemon())
      }
    })
}
