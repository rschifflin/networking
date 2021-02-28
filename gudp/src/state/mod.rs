use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use std::time::Instant;

use crossbeam::channel;
use mio::{Waker, Token};

use bring::bounded::Bring;
use cond_mutex::CondMutex;

use crate::types::FromDaemon as ToService;
use crate::types::TimerId;
use crate::socket::ConnOpts;

pub use status::Status;
pub use shared::Shared;

mod shared;
mod status;
mod events;

/// Connection state
/// Tracks all the behavior of a given connection
pub struct State {
  pub shared: Arc<Shared>,
  pub timer_id: TimerId,
  pub last_recv: Instant,
  pub fsm: FSM
}

pub enum FSM {
  Handshaking { conn_opts: ConnOpts },
  Connected
}
