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
    self.rx.recv().map(|conn| match conn {
      FromDaemon::Connection(shared) => Connection::new(self.waker, shared)
    }).map_err(error::cannot_recv_from_daemon)
  }
}

pub fn listen(service: &Service, socket: UdpSocket) -> io::Result<Listener> {
  let (tx, rx) = channel::bounded(1);
  let (tx_to_daemon, waker) = service.clone_parts();

  tx_to_daemon.send(ToDaemon::Listen(socket, tx))
    .map_err(error::cannot_send_to_daemon)?;

  waker.wake()?; // Force daemon to handle this new connection immediately
  Ok(Listener { rx, waker })
}
