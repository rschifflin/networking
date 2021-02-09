use std::sync::Arc;
use mio::{Poll, Waker};
use crossbeam::channel;

use crate::types::ToDaemon;
use crate::constants::WAKE_TOKEN;
use crate::daemon;

pub struct Service {
  waker: Arc<Waker>,
  to_daemon_tx: channel::Sender<ToDaemon>
}

impl Service {
  // Starts the service, spawning the daemon thread and providing access to connections
  pub fn initialize() -> Service {
    let (tx, other_rx) = channel::unbounded(); // Service -> Daemon
    let poll = Poll::new().expect("Could not make a poll");
    let waker = Arc::new(Waker::new(poll.registry(), WAKE_TOKEN).expect("Could not build new waker"));

    // Daemon thread
    daemon::spawn(poll, Arc::clone(&waker), other_rx);

    Service {
      waker,
      to_daemon_tx: tx
    }
  }

  pub fn print(&self, msg: &'static str) {
    self.to_daemon_tx.send(ToDaemon::Print(msg)).expect("Could not send debug print to gudp thread");
  }

  // Used internally to set up new connections but not part of the api
  pub(crate) fn clone_parts(&self) -> (channel::Sender<ToDaemon>, Arc<Waker>) {
    (self.to_daemon_tx.clone(), Arc::clone(&self.waker))
  }
}
