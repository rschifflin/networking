use crossbeam::channel;

use crate::types::SharedRingBuf;
use crate::types::FromDaemon as ToService;
use crate::constants::CONFIG_BUF_SIZE_BYTES;
use std::sync::{Arc, Condvar};

/// Connection state
/// Tracks all the behavior of a given socket
pub struct State {
  pub buf_read: SharedRingBuf,
  pub buf_write: SharedRingBuf,
  pub read_cond: Arc<Condvar>,

  pub buf_local: Vec<u8>,
  pub fsm: FSM
}

pub enum FSM {
  Listen { tx: channel::Sender<ToService> },
  Connected
}

impl State {
  pub fn init_connect(buf_read: SharedRingBuf, buf_write: SharedRingBuf, read_cond: Arc<Condvar>) -> State {
    State {
      buf_read,
      buf_write,
      read_cond,

      buf_local: vec![0u8; CONFIG_BUF_SIZE_BYTES],
      fsm: FSM::Connected
    }
  }

  pub fn init_listen(tx: channel::Sender<ToService>, buf_read: SharedRingBuf, buf_write: SharedRingBuf, read_cond: Arc<Condvar>) -> State {
    State {
      buf_read,
      buf_write,
      read_cond,

      buf_local: vec![0u8; CONFIG_BUF_SIZE_BYTES],
      fsm: FSM::Listen { tx }
    }
  }
}
