use std::time::Instant;

mod list;
// pub struct SystemClock();

pub use list::TimerList as List;
pub use list::Expired as Expired;

/// Monotonic non-decreasing clock
pub trait Clock {
  fn now(&self) -> Instant;
}

pub trait Timers<'a> {
  type Item: PartialEq + PartialOrd + Copy;
  type Expired: 'a + Iterator<Item=Self::Item>;

  fn add(&mut self, what: Self::Item, when: Instant);
  fn remove(&mut self, what: Self::Item, when: Instant);
  fn when_next(&self) -> Option<Instant>;

  // Advance the timers up to now.
  fn expire(&'a mut self, now: Instant) -> Self::Expired;
}

#[derive(Copy, Clone, Eq, Debug)]
pub enum TimerKind {
  Heartbeat,
  Timeout
}

impl PartialEq for TimerKind {
  fn eq(&self, other: &TimerKind) -> bool {
    (*self) as i32 == (*other) as i32
  }
}

impl PartialOrd for TimerKind {
  fn partial_cmp(&self, other: &TimerKind) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for TimerKind {
  fn cmp(&self, other: &TimerKind) -> std::cmp::Ordering {
    ((*self) as i32).cmp(&((*other) as i32))
  }
}
