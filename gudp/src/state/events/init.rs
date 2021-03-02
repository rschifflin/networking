use std::time::Instant;

use crate::socket::ConnOpts;
use crate::types::{Expired, TimerId};
use crate::state::{State, FSM, shared};
use crate::timer::{Timers, TimerKind};

impl State {
  pub fn init<'a, T>(when: Instant, timer_id: TimerId, timers: &mut T, conn_opts: ConnOpts) -> State
  where T: Timers<'a, Item = (TimerId, TimerKind), Expired = Expired<'a, T>> {
    timers.add(
      (timer_id, TimerKind::Timeout),
      when + std::time::Duration::from_millis(5_000));
    timers.add(
      (timer_id, TimerKind::Heartbeat),
      when + std::time::Duration::from_millis(1_000));

    State {
      shared: shared::new(),
      timer_id,
      last_recv: when,
      fsm: FSM::Handshaking { conn_opts }
    }
  }
}
