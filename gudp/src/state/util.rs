use crate::state::State;

impl State {
  pub fn on_io_error(&self, errno: Option<i32>) {
    let (ref buf_read, ref _buf_write, ref status, _) = *self.shared;
    let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
    status.set_io_err(errno);
    lock.notify_all();
  }
}
