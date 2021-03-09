use clock::Clock;

use crate::state::State;
use crate::timer::TimerKind;
use crate::daemon;
use crate::warn;

impl State {
  // Returns true when the connection is updated
  // Returns false when the connection has timed out
  pub fn timer<C: Clock>(&mut self, kind: TimerKind, s: &mut daemon::State<C>) -> bool {
    let (ref buf_read, ref buf_write, ref status) = *self.shared;
    match kind {
      TimerKind::Timeout => {
        let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
        status.set_peer_hup();
        lock.notify_all();
        self.clear_timers(s);
        false
      },

      TimerKind::Heartbeat => {
        let mut buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");
        let push_result = buf_write.push_back(&[]); // Heartbeat
        drop(buf_write);

        match push_result {
          Some(_size) => s.tx_on_write.send(self.socket_id).unwrap_or_else(warn::tx_to_write_send_failed),
          None => warn::prepare_heartbeat_failed()
        };

        true
      }
    }
  }
}
