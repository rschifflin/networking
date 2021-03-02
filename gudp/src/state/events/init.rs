use std::time::Instant;

use crate::socket::{self, ConnOpts};
use crate::state::{State, FSM, shared};
use crate::timer::{Timers, TimerKind};
use crate::daemon::LoopLocalState;

impl State {
  pub fn init(when: Instant, socket_id: socket::Id, conn_opts: ConnOpts, s: &mut LoopLocalState) -> State {
    s.timers.add(
      (socket_id, TimerKind::Timeout),
      when + std::time::Duration::from_millis(5_000));
    s.timers.add(
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
