use std::sync::{Arc, Mutex};
use std::io;

use crossbeam::channel;
use mio::net::UdpSocket as MioUdpSocket;
use mio::{Waker, Token};

use bring::bounded::Bring;
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
  pub fsm: FSM
}

pub enum FSM {
  Listen {
    token: Token,
    tx_to_service: channel::Sender<ToService>,
    tx_on_write: channel::Sender<Token>,
    waker: Arc<Waker>
  },
  Handshaking {
    token: Token,
    tx_to_service: channel::Sender<ToService>,
    tx_on_write: channel::Sender<Token>,
    waker: Arc<Waker>
  },
  Connected
}

impl State {
  // Returns None if unable to send the connection out to the client
  pub fn init_connect(
    token: Token,
    tx_to_service: channel::Sender<ToService>,
    tx_on_write: channel::Sender<Token>,
    waker: Arc<Waker>,
    io: &MioUdpSocket) -> io::Result<State> {

    // TODO: In reality, this will be a clock controlled heartbeat, not a one-off hello
    io.send(b"hello").map(|_| {
      State {
        shared: State::new_shared_state(),
        fsm: FSM::Handshaking { token, tx_to_service, tx_on_write, waker }
      }
    })
  }

  pub fn init_listen(
    token: Token,
    tx_to_service: channel::Sender<ToService>,
    tx_on_write: channel::Sender<Token>,
    waker: Arc<Waker>) -> State {
    State {
      shared: State::new_shared_state(),
      fsm: FSM::Listen { token, tx_to_service, tx_on_write, waker }
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
