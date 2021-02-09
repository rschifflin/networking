use crate::types::SharedRingBuf;
use crate::constants::CONFIG_BUF_SIZE_BYTES;

/// Connection state
/// Tracks all the behavior of a given socket
pub struct State {
  pub buf_read: SharedRingBuf,
  pub buf_write: SharedRingBuf,
  pub buf_local: Vec<u8>
}

impl State {
  pub fn new(buf_read: SharedRingBuf, buf_write: SharedRingBuf) -> State {
    State {
      buf_read,
      buf_write,
      buf_local: vec![0u8; CONFIG_BUF_SIZE_BYTES]
    }
  }
}