use std::net::SocketAddr;
use std::sync::atomic::Ordering::SeqCst as OSeqCst;

use crate::socket::{self, ConnOpts};
use crate::state::{State, FSM, Deps, Sequence, NetStat, shared};
use crate::timer::{Timers, TimerKind};
use crate::constants::time_ms;

impl State {
  pub fn init<D: Deps>(local_addr: SocketAddr, socket_id: socket::Id, conn_opts: ConnOpts, deps: &mut D) -> State {
    let when = deps.now();
    let timers = deps.timers();
    timers.add((socket_id, TimerKind::Timeout), when + time_ms::TIMEOUT);
    timers.add((socket_id, TimerKind::Heartbeat), when + time_ms::HEARTBEAT);
    let shared = shared::new();

    let rtt_ms = shared.3.rtt.load(OSeqCst);
    let netstat = NetStat::new(rtt_ms);

    // Notify that we have pending initial writes to send
    deps.notify_write(socket_id);

    State {
      shared: shared::new(),
      local_addr,
      socket_id,
      sequence: Sequence::new(), // TODO: Use deps.rand() to randomize seq no
      last_recv: when,
      last_send: when,
      netstat,
      fsm: FSM::Handshaking { conn_opts },
    }
  }
}
