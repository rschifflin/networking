use std::sync::Arc;
use std::io;
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
  pub fn initialize() -> io::Result<Service> {
    let (tx, other_rx) = channel::unbounded(); // Service -> Daemon
    let poll = Poll::new()?;
    let waker = Waker::new(poll.registry(), WAKE_TOKEN)?;
    let waker = Arc::new(waker);
    daemon::spawn(poll, Arc::clone(&waker), other_rx)?;

    Ok(Service { waker, to_daemon_tx: tx })
  }

  // Used internally to set up new connections but not part of the api
  pub(crate) fn clone_parts(&self) -> (channel::Sender<ToDaemon>, Arc<Waker>) {
    (self.to_daemon_tx.clone(), Arc::clone(&self.waker))
  }

  // TODO: Should we expose join at all? We should always drop and detach the daemon...
  // Maybe add an error reporting chan out of the daemon for observing errors?
}
