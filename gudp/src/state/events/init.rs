use std::time::Instant;

use crate::socket::ConnOpts;
use crate::types::TimerId;
use crate::state::{State, FSM, shared};
use crate::timer::{Expired, Timers};

impl State {
  // Returns None if unable to send the connection out to the client
  pub fn init<'a, T>(when: Instant, timer_id: TimerId, timers: &mut T, conn_opts: ConnOpts) -> State
  where T: Timers<'a, Expired<'a, TimerId>, TimerId> {
    timers.add(timer_id, std::time::Instant::now() + std::time::Duration::from_millis(1_000));

    State {
      shared: shared::new(),
      timer_id,
      last_recv: when,
      fsm: FSM::Handshaking { conn_opts }
    }
  }
}
