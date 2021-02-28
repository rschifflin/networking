use std::sync::{Arc, Mutex};

use bring::bounded::Bring;
use cond_mutex::CondMutex;

use crate::state::Status;
use crate::types::READ_BUFFER_TAG;
use crate::constants::CONFIG_BUF_SIZE_BYTES;

// TODO: Nominal type? Though ergonomics of destructuring tuple is nice...
pub type Shared = (
  /*BufRead*/   CondMutex<Bring, READ_BUFFER_TAG>,
  /*BufWrite*/  Mutex<Bring>,

  // Atomics
  /*Status*/    Status,
);

pub fn new() -> Arc<Shared> {
  let buf_read_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
  let buf_write_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
  let buf_read = CondMutex::new(Bring::from_vec(buf_read_vec));
  let buf_write = Mutex::new(Bring::from_vec(buf_write_vec));
  let status = Status::new();
  Arc::new((buf_read, buf_write, status))
}
