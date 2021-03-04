use std::net::{UdpSocket, ToSocketAddrs};
use std::sync::Arc;
use std::io;

use mio::{Poll, Waker};
use crossbeam::channel;

use clock::{Clock, SystemClock};

use log::warn;
use crate::types::{FromDaemon, ToDaemon};
use crate::constants::WAKE_TOKEN;
use crate::daemon;
use crate::Connection;
use crate::Listener;
use crate::error;

#[derive(Clone)]
pub struct Service {
  waker: Arc<Waker>,
  to_daemon_tx: channel::Sender<ToDaemon>
}

impl Service {
  // Starts the service, spawning the daemon thread and providing access to connections
  #[inline]
  pub fn initialize_with_clock<C: 'static + Clock + Send>(clock: C) -> io::Result<Service> {
    let (tx, other_rx) = channel::unbounded(); // Service -> Daemon
    let poll = Poll::new()?;
    let waker = Waker::new(poll.registry(), WAKE_TOKEN)?;
    let waker = Arc::new(waker);
    daemon::spawn(poll, Arc::clone(&waker), other_rx, clock)?;

    Ok(Service { waker, to_daemon_tx: tx })
  }

  pub fn initialize() -> io::Result<Service> {
    Self::initialize_with_clock(SystemClock())
  }

  pub fn connect<A: ToSocketAddrs>(&self, socket: UdpSocket, to_addr: A) -> io::Result<Connection> {
    let peer_addr = to_addr.to_socket_addrs().and_then(|mut addr| {
      addr.next()
        .map(Ok)
        .unwrap_or_else(|| Err(error::socket_addr_failed_to_resolve()))
    })?;

    let (tx, rx) = channel::bounded(2);
    let (tx_to_daemon, waker) = self.clone_parts();
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
        FromDaemon::Listener(on_close) => {
          warn!("When trying to register directly connected socket, received Listener instead");
          let listener = Listener::new(on_close, rx);
          drop(listener);
          Err(error::unexpected_recv_from_daemon())
        }
      })
  }

  pub fn listen(&self, socket: UdpSocket) -> io::Result<Listener> {
    let (tx, rx_from_daemon) = channel::bounded(2);
    let (tx_to_daemon, waker) = self.clone_parts();

    tx_to_daemon.send(ToDaemon::Listen(socket, tx))
      .map_err(error::cannot_send_to_daemon)?;

      waker.wake()?; // Force daemon to handle this new connection immediately

      match rx_from_daemon.recv() {
        // The expected case. Once the io has been confirmed, we can return a listener
        // which can accept() incoming connections.
        Ok(FromDaemon::Listener(on_close)) => {
          Ok(Listener::new(on_close, rx_from_daemon))
        },

        // This is unexpected. We only wanted a listener.
        // Close the given connection and signal the issue;
        Ok(FromDaemon::Connection(on_write, shared, id)) => {
          warn!("When trying to register listener socket, received direct connection instead");
          let conn = Connection::new(on_write, shared, id);
          drop(conn);
          Err(error::unexpected_recv_from_daemon())
        },

        // A closed rx means the daemon cannot register our io for some reason
        Err(_) => Err(error::cannot_register_with_daemon())
      }
  }

  pub fn wake(&self) -> io::Result<()> {
    self.waker.wake()
  }

  fn clone_parts(&self) -> (channel::Sender<ToDaemon>, Arc<Waker>) {
    (self.to_daemon_tx.clone(), Arc::clone(&self.waker))
  }

  // TODO: Should we expose join at all? We should always drop and detach the daemon...
  // Maybe add an error reporting chan out of the daemon for observing errors?
}
