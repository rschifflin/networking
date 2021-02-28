use std::net::SocketAddr;
use std::sync::Arc;
use std::io;
use std::time::Instant;

use crate::types::FromDaemon as ToService;
use crate::types::TimerId;
use crate::error;
use crate::state::{State, FSM};
use crate::timer::{Expired, Timers};

impl State {
  // Returns true when the connection is updated
  // Returns false when the connection is closed
  pub fn timer<'a, T>(&mut self, when: Instant, timers: &mut T) -> bool
  where T: Timers<'a, Expired<'a, TimerId>, TimerId> {
    let (ref buf_read, ref _buf_write, ref status) = *self.shared;

    // TODO: Should we handle a poisoned lock state here? IE if a thread with a connection panics,
    // what should the daemon do about it? Just close the connection?
    // Likely the client should panic on poison, and the daemon should recover the lock and close the conn on poison
    // For now just panic
    let mut buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");

    // NOTE: This status check must be done while holding the readlock to ensure no races occur of the form:
    // time0|thread0: client acquires the readlock.
    // time1|thread0: client observes an open status and no pending reads...
    // time2|thread1: open status changes to closed...
    // time3|thread1: this fn observes the closed status and signals the condvar to wake all _current_ sleepers
    // time4|thread0: ... and then client sleeps, oblivious to the change in status and condvar signal
    // Since after this notify_all, no future notifications are coming, client would sleep forever!
    // The solution is to prevent the client from acquiring the readlock until this fn observes the closed status.
    // That means when the client DOES acquire the readlock, it will observe a closed status and not sleep.
    // Alternatively, if the client acquired the readlock first, it will sleep on the condvar before this fn can notify.
    // Thus ensuring it will hear the notification to wake up and observe the closed status.
    if status.is_closed() {
      // TODO: Handle appropriate flushing behavior on closed ends:
      //    - if peer is closed, remove right away (app can still drain read buffer, app writes will fail)
      //    - if app is closed, discard reads but do not remove until writes are flushed
      //    - if io is closed, remove right away and remove all siblings
      buf.notify_all();
      return false;
    }

    if when.duration_since(self.last_recv) > std::time::Duration::from_millis(5_000) {
      status.set_io_hup();
      buf.notify_all();
      false
    } else {
      drop(buf);
      timers.add(self.timer_id, std::time::Instant::now() + std::time::Duration::from_millis(1_000));
      true
    }
  }
}
