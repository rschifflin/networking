use std::net::UdpSocket;
use std::sync::Arc;

use crossbeam::channel;
use mio::Waker;

use crate::Connection;
use crate::Service;
use crate::types::{FromDaemon, ToDaemon};

pub struct Listener {
  rx: channel::Receiver<FromDaemon>,
  waker: Arc<Waker>
}

impl Listener {
  // Block until connection is established or the daemon dies trying I guess
  // TODO: Result<Connection> in case any other msg or the daemon thread dying
  // TODO: What happens when listeners drop before calling accept?? And what _should_ happen ideally?
  pub fn accept(self) -> Option<Connection> {
    if let Ok(FromDaemon::Connection(shared)) = self.rx.recv() {
      return Some(Connection::new(self.waker, shared));
    }

    None
  }
}

pub fn listen(service: &Service, socket: UdpSocket) -> Listener {
  let (tx, rx) = channel::bounded(1);
  let (tx_to_daemon, waker) = service.clone_parts();
  tx_to_daemon.send(ToDaemon::Listen(socket, tx))
    .expect("Could not send new connection to daemon");

  waker.wake() // Force daemon to handle this new connection immediately
    .expect("Could not wake daemon to receive new connection");

  Listener { rx, waker }
}
