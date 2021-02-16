use std::sync::{Arc, Mutex};
use std::io;

use crossbeam::channel;
use mio::net::UdpSocket as MioUdpSocket;

use bring::Bring;
use cond_mutex::CondMutex;

use crate::types::SharedConnState;
use crate::types::FromDaemon as ToService;
use crate::constants::CONFIG_BUF_SIZE_BYTES;

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
  Handshaking { tx: channel::Sender<ToService> },
  Connected
}

impl State {
  // Returns None if unable to send the connection out to the client
  pub fn init_connect(tx: channel::Sender<ToService>, io: &MioUdpSocket) -> io::Result<State> {
    // TODO: In reality, this will be a clock controlled heartbeat, not a one-off hello
    io.send(b"hello").map(|_| {
      State {
        shared: State::new_shared_state(),
        buf_local: vec![0u8; CONFIG_BUF_SIZE_BYTES],
        fsm: FSM::Handshaking { tx }
      }
    })
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
    let status = Status::new();
    Arc::new((buf_read, buf_write, status))
  }
}
