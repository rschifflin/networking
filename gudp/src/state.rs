use crossbeam::channel;

use crate::types::SharedConnState;
use crate::types::FromDaemon as ToService;
use crate::constants::CONFIG_BUF_SIZE_BYTES;
use std::sync::Arc;

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
  pub fn init_connect(shared: Arc<SharedConnState>) -> State {
    State {
      shared,
      buf_local: vec![0u8; CONFIG_BUF_SIZE_BYTES],
      fsm: FSM::Connected
    }
  }

  pub fn init_listen(tx: channel::Sender<ToService>, shared: Arc<SharedConnState>) -> State {
    State {
      shared,
      buf_local: vec![0u8; CONFIG_BUF_SIZE_BYTES],
      fsm: FSM::Listen { tx }
    }
  }
}
