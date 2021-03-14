use std::sync::Arc;
use std::time::Instant;
use std::net::SocketAddr;

use crate::socket::{self, ConnOpts};

pub use status::Status;
pub use shared::Shared;
pub use deps::Deps;
use sequence::{Sequence, SentSeqNo};

mod sequence;
mod shared;
mod status;
mod events;
mod util;
mod deps;

/// Connection state
/// Tracks all the behavior of a given connection
pub struct State {
  pub shared: Arc<Shared>,
  pub local_addr: SocketAddr,
  pub socket_id: socket::Id,
  pub last_recv: Instant,
  pub last_send: Instant,
  pub sequence: Sequence,
  pub fsm: FSM
}

pub enum FSM {
  Handshaking { conn_opts: ConnOpts },
  Connected
}
