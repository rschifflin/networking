use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use std::time::Instant;

use crossbeam::channel;
use mio::{Waker, Token};

use bring::bounded::Bring;
use cond_mutex::CondMutex;

use crate::types::SharedConnState;
use crate::types::FromDaemon as ToService;
use crate::constants::CONFIG_BUF_SIZE_BYTES;
use crate::socket::ConnOpts;

pub use status::Status;
mod status;
mod read;
mod write;

/// Connection state
/// Tracks all the behavior of a given socket
pub struct State {
  pub shared: Arc<SharedConnState>,
  pub last_recv: Instant,
  pub fsm: FSM
}

pub enum FSM {
  Handshaking {
    conn_opts: ConnOpts,
  },
  Connected
}

impl State {
  // Returns None if unable to send the connection out to the client
  pub fn init_connect(when: Instant, conn_opts: ConnOpts) -> State {

    State {
      shared: State::new_shared_state(),
      last_recv: when,
      fsm: FSM::Handshaking { conn_opts }
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
