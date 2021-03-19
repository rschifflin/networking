use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use super::Clock as ClockT;

#[derive(Clone)]
pub struct Clock {
  now: Arc<Mutex<Instant>>
}

impl Clock {
  pub fn new(now: Instant) -> Clock {
    Clock {
      now: Arc::new(Mutex::new(now))
    }
  }

  pub fn tick_ms(&self, amount_ms: u64) {
    let mut now = self.now.lock().expect("Could not acquire unpoisoned test clock mutex");
    *now = *now + Duration::from_millis(amount_ms);
  }
}

impl ClockT for Clock {
  fn now(&self) -> Instant {
    let now = self.now.lock().expect("Could not acquire unpoisoned test clock mutex");
    *now
  }
}
