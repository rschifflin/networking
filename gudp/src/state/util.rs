use std::time::{Instant, Duration};

use crate::constants::time_ms;
use crate::timer::{self, Timers, TimerKind};
use crate::state::{State, Deps};
use crate::socket;

impl State {
  pub fn on_io_error<D: Deps>(&self, errno: Option<i32>, deps: &mut D) {
    let (ref buf_read, ref _buf_write, ref status, _) = *self.shared;
    let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
    status.set_io_err(errno);
    lock.notify_all();
  }
}
