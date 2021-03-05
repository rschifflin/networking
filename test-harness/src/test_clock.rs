use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use clock::Clock;

#[derive(Clone)]
pub struct TestClock {
  now: Arc<Mutex<Instant>>
}

impl TestClock {
  pub fn new(now: Instant) -> TestClock {
    TestClock {
      now: Arc::new(Mutex::new(now))
    }
  }

  pub fn tick_ms(&self, amount_ms: u64) {
    let mut now = self.now.lock().expect("Could not acquire unpoisoned test clock mutex");
    *now = *now + Duration::from_millis(amount_ms);
  }
}

impl Clock for TestClock {
  fn now(&self) -> Instant {
    let now = self.now.lock().expect("Could not acquire unpoisoned test clock mutex");
    *now
  }
}
