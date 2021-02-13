use std::net::UdpSocket;
use std::sync::Arc;
use std::io;

use crossbeam::channel;
use mio::Waker;

use crate::error;
use crate::Connection;
use crate::Service;
use crate::types::{FromDaemon, ToDaemon};

pub struct Listener {
  rx: channel::Receiver<FromDaemon>,
  waker: Arc<Waker>
}

impl Listener {
  // Block until connection is established or the daemon dies trying I guess
  // TODO: What happens when listeners drop before calling accept?? And what _should_ happen ideally?
  pub fn accept(self) -> io::Result<Connection> {
    match self.rx.recv() {
      Ok(FromDaemon::Connection(shared)) => Ok(Connection::new(self.waker, shared)),
      Err(e) => Err(error::cannot_recv_from_daemon(e)),
      _ => Err(error::unexpected_recv_from_daemon())
    }
  }
}

pub fn listen(service: &Service, socket: UdpSocket) -> io::Result<Listener> {
  let (tx, rx_from_daemon) = channel::bounded(2);
  let (tx_to_daemon, waker) = service.clone_parts();

  tx_to_daemon.send(ToDaemon::Listen(socket, tx))
    .map_err(error::cannot_send_to_daemon)?;

  match rx_from_daemon.recv() {
    // The expected case. Once the io has been confirmed, we can return a listener
    // which can accept() incoming connections.
    Ok(FromDaemon::IORegistered) => {
      waker.wake()?; // Force daemon to handle this new connection immediately
      Ok(Listener { rx: rx_from_daemon, waker })
    },

    // This is unexpected. We only wanted an IORegistered message.
    // Close the given connection and signal the issue;
    Ok(FromDaemon::Connection(shared)) => {
      let conn = Connection::new(waker, shared);
      drop(conn);
      Err(error::unexpected_recv_from_daemon())
    },

    // A closed rx means the daemon cannot register our io for some reason
    Err(_) => Err(error::cannot_register_with_daemon())
  }
}
