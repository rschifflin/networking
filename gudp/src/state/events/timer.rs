use crate::state::{State, Deps};
use crate::constants::time_ms;
use crate::timer::{Timers, TimerKind};

impl State {
  // Returns true when the connection is updated
  // Returns false when the connection has timed out
  pub fn timer<D: Deps>(&mut self, kind: TimerKind, deps: &mut D) -> bool {
    let (ref buf_read, _, ref status, _) = *self.shared;
    match kind {
      TimerKind::Timeout => {
        let when = deps.now();
        if (when - self.last_recv) >= time_ms::TIMEOUT {
          let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
          status.set_peer_hup();
          lock.notify_all();
          false
        } else {
          deps.timers().add((self.socket_id, TimerKind::Timeout), when + time_ms::TIMEOUT);
          true
        }
      },

      TimerKind::Heartbeat => {
        let when = deps.now();
        deps.timers().add((self.socket_id, TimerKind::Heartbeat), when + time_ms::HEARTBEAT);
        deps.notify_write(self.socket_id);

        true
      }
    }
  }
}
