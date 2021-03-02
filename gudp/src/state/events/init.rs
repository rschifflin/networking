use std::time::Instant;

use crate::socket::{self, ConnOpts};
use crate::types::Expired;
use crate::state::{State, FSM, shared};
use crate::timer::{Timers, TimerKind};

impl State {
  pub fn init<'a, T>(when: Instant, socket_id: socket::Id, timers: &mut T, conn_opts: ConnOpts) -> State
  where T: Timers<'a, Item = (socket::Id, TimerKind), Expired = Expired<'a, T>> {
    timers.add(
      (socket_id, TimerKind::Timeout),
      when + std::time::Duration::from_millis(5_000));
    timers.add(
      (socket_id, TimerKind::Heartbeat),
      // TODO: Move zero-duration into constants
      when + std::time::Duration::from_millis(0));

    State {
      shared: shared::new(),
      socket_id,
      last_recv: when,
      last_send: when,
      fsm: FSM::Handshaking { conn_opts }
    }
  }
}
