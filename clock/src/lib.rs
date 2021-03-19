use std::time::Instant;

pub mod sys;
pub mod mock;

/// Monotonic non-decreasing clock
pub trait Clock {
  fn now(&self) -> Instant;
}
