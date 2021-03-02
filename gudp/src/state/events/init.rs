use crate::socket::{self, ConnOpts};
use crate::state::{State, FSM, shared};
use crate::timer::{Timers, TimerKind, Clock};
use crate::daemon::LoopLocalState;
use crate::constants::time_ms;

impl State {
  pub fn init(socket_id: socket::Id, conn_opts: ConnOpts, s: &mut LoopLocalState) -> State {
    let when = s.clock.now();
    s.timers.add((socket_id, TimerKind::Timeout), when + time_ms::T_5000);
    s.timers.add((socket_id, TimerKind::Heartbeat), when + time_ms::ZERO);

    State {
      shared: shared::new(),
      socket_id,
      last_recv: when,
      last_send: when,
      fsm: FSM::Handshaking { conn_opts }
    }
  }
}
