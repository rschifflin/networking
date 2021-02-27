use std::time::Instant;

mod list;
// pub struct SystemClock();

pub use list::TimerList as List;

/// Monotonic non-decreasing clock
pub trait Clock {
  fn now(&self) -> Instant;
}

pub trait Timers<'a,
    I: 'a + Iterator<Item=T>,
    T: PartialEq + PartialOrd + Copy> {

  fn add(&mut self, what: T, when: Instant);
  fn remove(&mut self, what: T, when: Instant);
  fn when_next(&self) -> Option<Instant>;

  // Advance the timers up to now.
  fn expire(&'a mut self, now: Instant) -> I;
}
