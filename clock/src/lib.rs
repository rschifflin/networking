use std::time::Instant;

/// Monotonic non-decreasing clock
pub trait Clock {
  fn now(&self) -> Instant;
}

pub struct SystemClock();

impl Clock for SystemClock {
  fn now(&self) -> Instant {
    Instant::now()
  }
}
