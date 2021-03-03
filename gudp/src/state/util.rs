use crate::daemon::LoopLocalState;
use crate::constants::time_ms;
use crate::timer::{Timers, TimerKind};
use crate::state::State;

impl State {
  pub(crate) fn clear_timers(&self, s: &mut LoopLocalState) {
    s.timers.remove((self.socket_id, TimerKind::Heartbeat), self.last_send + time_ms::HEARTBEAT);
    s.timers.remove((self.socket_id, TimerKind::Timeout), self.last_recv + time_ms::TIMEOUT);
  }

  pub fn on_io_error(&self, errno: Option<i32>, s: &mut LoopLocalState) {
    self.clear_timers(s);
    let (ref buf_read, ref _buf_write, ref status) = *self.shared;
    let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
    status.set_io_err(errno);
    lock.notify_all();
  }
}
