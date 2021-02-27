use std::time::Instant;
use super::Timers;

pub struct TimerList<T: Ord + Copy> {
  timers: Vec<(Instant, T, bool)>
}

pub struct Expired<'a, T> {
  items: std::vec::Drain<'a, (Instant, T, bool)>
}

impl<'a, T> Iterator for Expired<'a, T> {
  type Item = T;
  fn next(&mut self) -> Option<T> {
    loop {
      match self.items.next() {
        Some((_, item, is_live)) => {
          if is_live { return Some(item) }
        },
        None => return None
      }
    }
  }
}

impl<T: Ord + Copy> TimerList<T> {
  pub fn new() -> TimerList<T> {
    TimerList {
      timers: vec![],
    }
  }

  fn find(&self, what: T, when: Instant) -> Result<usize, usize> {
    self.timers.binary_search_by(|&(ref existing_when, ref existing_what, _)| {
      match when.cmp(existing_when) {
        std::cmp::Ordering::Equal => match what.cmp(existing_what) {
          std::cmp::Ordering::Less => std::cmp::Ordering::Greater,
          std::cmp::Ordering::Greater => std::cmp::Ordering::Less,
          equal => equal
        },
        std::cmp::Ordering::Less => std::cmp::Ordering::Greater,
        std::cmp::Ordering::Greater => std::cmp::Ordering::Less
      }
    }, )
  }

  fn find_when(&self, when: Instant) -> Result<usize, usize> {
    self.timers.binary_search_by(|&(ref existing_when, _, _)| {
      match when.cmp(existing_when) {
        std::cmp::Ordering::Less => std::cmp::Ordering::Greater,
        _gt_or_eq => std::cmp::Ordering::Less
      }
    })
  }
}

impl<'a, T> Timers<'a, Expired<'a, T>, T> for TimerList<T>
where T: Ord + Copy {
  fn add(&mut self, what: T, when: Instant) {
    self.find(what, when).map_err(|idx| {
      self.timers.insert(idx, (when, what, true))
    }).ok();
  }

  fn remove(&mut self, what: T, when: Instant) {
    self.find(what, when).map(|idx| {
      self.timers[idx].2 = false;
    }).ok();
  }

  fn when_next(&self) -> Option<Instant> {
    self.timers.iter().filter_map(|(when, _, is_live)| {
      if *is_live {
        Some(*when)
      } else {
        None
      }
    }).next()
  }

  fn expire(&mut self, now: Instant) -> Expired<T> {
    let range_end = self.find_when(now).unwrap_or_else(|idx| idx);
    Expired { items: self.timers.drain(..range_end) }
  }
}

#[cfg(test)]
mod tests {
  use crate::timer::Timers;
  use super::TimerList;
  use std::time::{Duration, Instant};

  #[test]
  fn empty_timer() {
    let timers: TimerList<usize> = super::TimerList::new();
    assert_eq!(timers.when_next(), None);
  }

  #[test]
  fn sorted_soonest_to_furthest() {
    let mut timers: TimerList<usize> = super::TimerList::new();
    let jiffy = Duration::from_millis(500);
    let first = Instant::now() + jiffy;
    let second = first + jiffy;
    let third = second + jiffy;
    timers.add(2, third);
    timers.add(1, second);
    timers.add(0, first);

    assert_eq!(timers.when_next(), Some(first));
  }

  #[test]
  fn next_skips_deletes() {
    let mut timers: TimerList<usize> = super::TimerList::new();
    let jiffy = Duration::from_millis(500);
    let first = Instant::now() + jiffy;
    let second = first + jiffy;
    let third = second + jiffy;
    timers.add(0, first);
    timers.add(1, second);
    timers.add(2, third);
    timers.remove(0, first);
    timers.remove(1, second);

    assert_eq!(timers.when_next(), Some(third));
  }

  #[test]
  fn iteration() {
    let mut timers: TimerList<usize> = super::TimerList::new();
    let jiffy = Duration::from_millis(500);
    let first = Instant::now() + jiffy;
    let second = first + jiffy;
    let third = second + jiffy;
    let fourth = third + jiffy;
    let fifth = fourth + jiffy;
    timers.add(0, first);
    timers.add(1, second);
    timers.add(2, third);
    timers.add(3, third);
    timers.add(4, third);
    timers.add(5, fourth);
    timers.add(5, fourth); // Intentional repeat is ignored
    timers.add(6, fourth);
    timers.add(7, fifth);
    timers.add(8, fifth);

    let expired: Vec<usize> = timers.expire(fourth).collect();
    assert_eq!(expired, vec![0,1,2,3,4,5,6]);
  }

  #[test]
  fn iteration_before_any_expire() {
    let mut timers: TimerList<usize> = super::TimerList::new();
    let jiffy = Duration::from_millis(500);
    let now = Instant::now();
    let first = now + jiffy;
    let second = first + jiffy;
    let third = second + jiffy;
    let fourth = third + jiffy;
    let fifth = fourth + jiffy;
    timers.add(0, first);
    timers.add(1, second);
    timers.add(2, third);
    timers.add(3, third);
    timers.add(4, third);
    timers.add(5, fourth);
    timers.add(6, fourth);
    timers.add(7, fifth);
    timers.add(8, fifth);

    let expired: Vec<usize> = timers.expire(now).collect();
    assert_eq!(expired, vec![]);
  }

}
