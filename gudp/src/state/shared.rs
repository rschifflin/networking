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

fn initial_write_ring_buf() -> Bring {
  let buf_write_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
  let mut ring_buf = Bring::from_vec(buf_write_vec);
  ring_buf.push_back(&[]).expect("Could not initialize write ring buffer with ping data!");
  ring_buf
}

fn initial_read_ring_buf() -> Bring {
  let buf_read_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
  Bring::from_vec(buf_read_vec)
}

pub fn new() -> Arc<Shared> {
  let buf_read = CondMutex::new(initial_read_ring_buf());
  let buf_write = Mutex::new(initial_write_ring_buf());
  let status = Status::new();
  Arc::new((buf_read, buf_write, status))
}
