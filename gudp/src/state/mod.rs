use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicUsize;

use crossbeam::channel;

use bring::Bring;

use crate::types::SharedConnState;
use crate::types::FromDaemon as ToService;
use crate::constants::CONFIG_BUF_SIZE_BYTES;
use crate::sync::CondMutex;

pub use status::Status;
mod status;

/// Connection state
/// Tracks all the behavior of a given socket
pub struct State {
  pub shared: Arc<SharedConnState>,
  pub buf_local: Vec<u8>,
  pub fsm: FSM
}

pub enum FSM {
  Listen { tx: channel::Sender<ToService> },
  Connected
}

impl State {
  pub fn init_connect(tx: channel::Sender<ToService>) -> State {
    let shared = State::new_shared_state();

    tx.send(
      ToService::Connection(Arc::clone(&shared))
    ).expect("Could not respond with connection state");

    State {
      shared,
      buf_local: vec![0u8; CONFIG_BUF_SIZE_BYTES],
      fsm: FSM::Connected
    }
  }

  pub fn init_listen(tx: channel::Sender<ToService>) -> State {
    State {
      shared: State::new_shared_state(),
      buf_local: vec![0u8; CONFIG_BUF_SIZE_BYTES],
      fsm: FSM::Listen { tx }
    }
  }

  fn new_shared_state() -> Arc<SharedConnState> {
    let buf_read_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
    let buf_write_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
    let buf_read = CondMutex::new(Bring::from_vec(buf_read_vec));
    let buf_write = Mutex::new(Bring::from_vec(buf_write_vec));
    let status = Status::new(AtomicUsize::new(0));
    Arc::new((buf_read, buf_write, status))
  }
}
