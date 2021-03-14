use mio::Token;

pub const CONFIG_BUF_SIZE_BYTES: usize = 4096;
pub const WAKE_TOKEN: Token = Token(0);
pub const SENT_SEQ_BUF_SIZE: usize = 1024;

pub mod header {
  use core::ops::Range;
  pub const MAGIC_BYTES: [u8; 4] = 0xdeadbeef_u32.to_be_bytes();
  pub const MAGIC_BYTES_RANGE: Range<usize> =
    0..MAGIC_BYTES.len();

  pub const LOCAL_SEQ_NO_SIZE_BYTES: usize = 4;
  pub const LOCAL_SEQ_NO_OFFSET: usize = 4;
  pub const LOCAL_SEQ_NO_RANGE: Range<usize> =
    LOCAL_SEQ_NO_OFFSET..LOCAL_SEQ_NO_OFFSET + LOCAL_SEQ_NO_SIZE_BYTES;

  pub const REMOTE_SEQ_NO_SIZE_BYTES: usize = 4;
  pub const REMOTE_SEQ_NO_OFFSET: usize = 8;
  pub const REMOTE_SEQ_NO_RANGE: Range<usize> =
    REMOTE_SEQ_NO_OFFSET..REMOTE_SEQ_NO_OFFSET + REMOTE_SEQ_NO_SIZE_BYTES;

  pub const REMOTE_SEQ_TAIL_SIZE_BYTES: usize = 4;
  pub const REMOTE_SEQ_TAIL_OFFSET: usize = 12;
  pub const REMOTE_SEQ_TAIL_RANGE: Range<usize> =
    REMOTE_SEQ_TAIL_OFFSET..REMOTE_SEQ_TAIL_OFFSET + REMOTE_SEQ_TAIL_SIZE_BYTES;

// magic bytes + local seq + remote seq + remote seq tail
  pub const SIZE_BYTES: usize =
    MAGIC_BYTES.len() +
    LOCAL_SEQ_NO_SIZE_BYTES +
    REMOTE_SEQ_NO_SIZE_BYTES +
    REMOTE_SEQ_TAIL_SIZE_BYTES;
}

pub mod time_ms {
  use std::time::Duration;

  pub const ZERO: Duration = Duration::from_millis(0);
  pub const IOTA: Duration = Duration::from_millis(10);
  pub const HEARTBEAT: Duration = Duration::from_millis(1_000);
  pub const TIMEOUT: Duration = Duration::from_millis(15_000);
}
