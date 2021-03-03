use mio::Token;

pub const CONFIG_BUF_SIZE_BYTES: usize = 4096;
pub const WAKE_TOKEN: Token = Token(0);

pub mod time_ms {
  use std::time::Duration;

  pub const ZERO: Duration = Duration::from_millis(0);
  pub const IOTA: Duration = Duration::from_millis(10);
  pub const HEARTBEAT: Duration = Duration::from_millis(1_000);
  pub const TIMEOUT: Duration = Duration::from_millis(5_000);
}
