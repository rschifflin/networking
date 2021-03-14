use std::time::Instant;

use crate::constants::time_ms;
use crate::timer::{self, Timers, TimerKind};
use crate::state::{State, Deps};
use crate::socket;

impl State {
  pub(crate) fn clear_timers(&self, timers: &mut timer::List<(socket::Id, TimerKind)>) {
    timers.remove((self.socket_id, TimerKind::Heartbeat), self.last_send + time_ms::HEARTBEAT);
    timers.remove((self.socket_id, TimerKind::Timeout), self.last_recv + time_ms::TIMEOUT);
  }

  pub fn on_io_error<D: Deps>(&self, errno: Option<i32>, deps: &mut D) {
    self.clear_timers(deps.timers());
    let (ref buf_read, ref _buf_write, ref status) = *self.shared;
    let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
    status.set_io_err(errno);
    lock.notify_all();
  }
}

pub fn bump_timeout(socket_id: socket::Id, last_recv: &mut Instant, when: Instant, timers: &mut timer::List<(socket::Id, TimerKind)>) {
  timers.remove((socket_id, TimerKind::Timeout), *last_recv + time_ms::TIMEOUT);
  *last_recv = when;
  timers.add((socket_id, TimerKind::Timeout), when + time_ms::TIMEOUT);
}

pub fn bump_heartbeat(socket_id: socket::Id, last_send: &mut Instant, when: Instant, timers: &mut timer::List<(socket::Id, TimerKind)>) {
  timers.remove((socket_id, TimerKind::Heartbeat), *last_send + time_ms::HEARTBEAT);
  *last_send = when;
  timers.add((socket_id, TimerKind::Heartbeat), when + time_ms::HEARTBEAT);
}
