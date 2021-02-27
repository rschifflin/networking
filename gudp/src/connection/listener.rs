use std::net::UdpSocket;
use std::io;

use crossbeam::channel;
use log::warn;

use crate::error;
use crate::Connection;
use crate::Service;
use crate::types::{FromDaemon, ToDaemon, OnClose};

pub struct Listener {
  on_close: Box<OnClose>,
  pub rx: channel::Receiver<FromDaemon>,
}

impl Drop for Listener {
  fn drop(&mut self) {
    (self.on_close)().map(|_| {
      // NOTE: This is a blocking call, waiting for the daemon thread to close its sender.
      // The reason being, a try_iter loop would be racy-
      //   it's possible after we iterate but before we drop, a new item is sent.
      //   that item would be lost, and the io on the daemon side would never be cleaned up.
      // But it can be very counter-intuitive for a Destructor to block!!
      // We might want a recv_timeout here just in case something strange occurs.
      for awaiting_conn in self.rx.iter() {
        if let FromDaemon::Connection(on_write, shared, id) = awaiting_conn {
          // Build and drop any awaiting connections until the daemon closes its sender
          // Dropping a connection guarantees it will eventually be cleaned up
          Connection::new(on_write, shared, id);
        }
      }
    }).unwrap_or_else(|e| { warn!("Could not close listening socket on drop: {}. Resource may leak!", e); });
  }
}

impl Listener {
  pub fn new(on_close: Box<OnClose>, rx: channel::Receiver<FromDaemon>) -> Listener {
    Listener { on_close, rx }
  }
  // Block until connection is established or the daemon dies trying I guess
  // TODO: What happens when listeners drop before calling accept?? And what _should_ happen ideally?
  pub fn accept(&self) -> io::Result<Connection> {
    match self.rx.recv() {
      Ok(FromDaemon::Connection(on_write, shared, id)) => Ok(Connection::new(on_write, shared, id)),
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

  waker.wake()?; // Force daemon to handle this new connection immediately

  match rx_from_daemon.recv() {
    // The expected case. Once the io has been confirmed, we can return a listener
    // which can accept() incoming connections.
    Ok(FromDaemon::Listener(on_close)) => {
      Ok(Listener {
        on_close,
        rx: rx_from_daemon
      })
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
