use std::sync::Arc;
use std::time::Instant;

use crate::socket::{self, ConnOpts};

pub use status::Status;
pub use shared::Shared;

mod shared;
mod status;
mod events;

/// Connection state
/// Tracks all the behavior of a given connection
pub struct State {
  pub shared: Arc<Shared>,
  pub socket_id: socket::Id,
  pub last_recv: Instant,
  pub last_send: Instant,
  pub fsm: FSM
}

pub enum FSM {
  Handshaking { conn_opts: ConnOpts },
  Connected
}
