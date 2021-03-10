use crate::state::{State, Deps};
use crate::timer::TimerKind;
use crate::warn;

impl State {
  // Returns true when the connection is updated
  // Returns false when the connection has timed out
  pub fn timer<D: Deps>(&mut self, kind: TimerKind, deps: &mut D) -> bool {
    let (ref buf_read, ref buf_write, ref status) = *self.shared;
    match kind {
      TimerKind::Timeout => {
        let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
        status.set_peer_hup();
        lock.notify_all();
        self.clear_timers(deps.timers());
        false
      },

      TimerKind::Heartbeat => {
        let mut buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");
        let push_result = buf_write.push_back(&[]); // Heartbeat
        drop(buf_write);

        match push_result {
          Some(_size) => deps.notify_write(self.socket_id),
          None => warn::prepare_heartbeat_failed()
        };

        true
      }
    }
  }
}
