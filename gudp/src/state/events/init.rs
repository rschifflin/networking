use std::net::SocketAddr;

use clock::Clock;

use crate::socket::{self, ConnOpts};
use crate::state::{State, FSM, shared};
use crate::timer::{Timers, TimerKind};
use crate::daemon;
use crate::constants::time_ms;
use crate::warn;

impl State {
  pub fn init<C: Clock>(local_addr: SocketAddr, socket_id: socket::Id, conn_opts: ConnOpts, s: &mut daemon::State<C>) -> State {
    let when = s.clock.now();
    s.timers.add((socket_id, TimerKind::Timeout), when + time_ms::TIMEOUT);
    s.timers.add((socket_id, TimerKind::Heartbeat), when + time_ms::HEARTBEAT);

    // Notify that we have pending initial writes to send
    s.tx_on_write.send(socket_id).unwrap_or_else(warn::tx_to_write_send_failed);

    State {
      shared: shared::new(),
      local_addr,
      socket_id,
      last_recv: when,
      last_send: when,
      fsm: FSM::Handshaking { conn_opts }
    }
  }
}
