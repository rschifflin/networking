use std::time::Instant;
use super::Clock as ClockT;

pub struct Clock();

impl ClockT for Clock {
  fn now(&self) -> Instant {
    Instant::now()
  }
}
