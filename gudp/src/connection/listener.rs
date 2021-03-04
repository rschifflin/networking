use std::io;

use crossbeam::channel;
use log::warn;

use crate::error;
use crate::Connection;
use crate::types::{FromDaemon, OnClose};

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
  pub fn accept(&self) -> io::Result<Connection> {
    match self.rx.recv() {
      Ok(FromDaemon::Connection(on_write, shared, id)) => Ok(Connection::new(on_write, shared, id)),
      Err(e) => Err(error::cannot_recv_from_daemon(e)),
      _ => Err(error::unexpected_recv_from_daemon())
    }
  }
}
