use std::time::Instant;

use log::trace;

use crate::constants::SENT_SEQ_BUF_SIZE;
pub type SeqNo = u32;

const MAX_MINUS_31: u32 = u32::MAX - 31;
const MAX_MINUS_32: u32 = u32::MAX - 31;
const HALF_MAX_PLUS_1: u32 = u32::MAX/2 + 1;

#[derive(Copy, Clone)]
pub struct SentSeqNo {
  pub seq_no: SeqNo,
  pub acked: bool,
  pub when: Instant
}

pub struct Sequence {
  pub local_seq_no: SeqNo,  // Represents the next seq no to send
  pub remote_seq_no: SeqNo, // Represents the last seq no to recv
  pub remote_seq_tail: u32, // Represents a redundant tail of 32 seq nos received, relative to the remote seq no

  pub sent_seq_buf: Vec<Option<SentSeqNo>>,
}

impl Sequence {
  pub fn new() -> Sequence {
    Sequence {
      local_seq_no: 0,
      remote_seq_no: 0,
      remote_seq_tail: 0,
      sent_seq_buf: vec![None; SENT_SEQ_BUF_SIZE]
    }
  }

  pub fn update_remote(&mut self, seq_no: SeqNo, seq_gap: u32) {
    // If the gap is >= 32, simply set the remote tail to 0
    // If the gap is < 32, left-shift by 1, set LSB to 1, then left-shift by GAP - 1

    match seq_gap {
      0 => return,
      1..=31 => {
        let mut tail = self.remote_seq_tail << 1;
        tail |= 1;
        self.remote_seq_tail = tail << (seq_gap - 1);
      },
      _else_gte_32 => self.remote_seq_tail = 0
    }

    self.remote_seq_no = seq_no;
  }

  // Removes all sequence numbers no longer ackable following this gap
  // Returns the # removed that were unacked
  pub fn clear_old(&mut self, seq_gap: u32) -> u32 {
    let edge = self.remote_seq_no.wrapping_sub(32);
    let mut unacked = 0;
    for idx in 0..seq_gap {
      let expected_seq_no = edge.wrapping_add(idx);
      let sent_idx = expected_seq_no as usize % SENT_SEQ_BUF_SIZE;

      if let Some(mut ssn) = self.sent_seq_buf[sent_idx] {
        if ssn.seq_no == expected_seq_no && !ssn.acked {
          unacked += 1;
          self.sent_seq_buf[sent_idx] = None;
        }
      }
    }

    unacked
  }

  pub fn iter_acks(&mut self, seq_no: SeqNo, seq_tail: u32) -> AckIter {
    AckIter::new(self, seq_no, seq_tail)
  }
}

impl SentSeqNo {
  pub fn new(seq_no: SeqNo, when: Instant) -> SentSeqNo {
    SentSeqNo {
      seq_no,
      acked: false,
      when
    }
  }
}

pub struct AckIter<'a> {
  ack_seq_no: SeqNo,
  ack_seq_tail: u32,
  acks_remaining: u32,

  sequence: &'a mut Sequence,
}

impl<'a> AckIter<'a> {
  pub fn new(sequence: &'a mut Sequence, ack_seq_no: SeqNo, ack_seq_tail: u32) -> AckIter<'a> {
    AckIter {
      ack_seq_no,
      ack_seq_tail,
      acks_remaining: 33, // The sequence number plus a 32-bit tail
      sequence
    }
  }
}

impl Iterator for AckIter<'_> {
  type Item = SentSeqNo;
  fn next(&mut self) -> Option<SentSeqNo> {
    loop {
      match (self.ack_seq_tail, self.acks_remaining) {
        // All exhausted
        (0, 0) => return None,

        // tail exhausted, ack the absolute seq no
        (0, _) => {
          self.acks_remaining = 0;
          let ack_seq_no = self.ack_seq_no;
          let idx = ack_seq_no as usize % SENT_SEQ_BUF_SIZE;
          if let Some(mut sent) = self.sequence.sent_seq_buf[idx].as_mut() {
            if sent.seq_no == ack_seq_no && !sent.acked {
              sent.acked = true;
              return Some(*sent);
            }
          }
        },

        // ack next seq no in tail relative to absolute seq no
        (_, _) => {
          let ack_seq_tail = self.ack_seq_tail;
          self.ack_seq_tail <<= 1;
          self.acks_remaining -= 1;

          if (ack_seq_tail & 0b10000000_00000000_00000000_00000000) > 0 {
            let ack_seq_no = self.ack_seq_no.wrapping_sub(self.acks_remaining);
            let idx = ack_seq_no as usize % SENT_SEQ_BUF_SIZE;
            if let Some(mut sent) = self.sequence.sent_seq_buf[idx].as_mut() {
              if sent.seq_no == ack_seq_no && !sent.acked {
                sent.acked = true;
                return Some(*sent);
              }
            }
          }
        }
      }
    }
  }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Distance {
  Old,
  Redundant,
  New(u32)
}

// Returns the distance from start to end if newer
// Returns Redundant if zero or within 32 of the newest
// Returns Old if more than 32 older than newest

// NOTE: As sequence numbers wrap around, the end magnitude may be less than the start,
// but the distance remains positive. Distance is relative to the start, up to u32::max/2 ahead.
pub fn distance(start: SeqNo, end: SeqNo) -> Distance {
  let redundant_min = u32::MAX - 31;
  let old_min = u32::MAX/2;
  match end.wrapping_sub(start) {
    0 => Distance::Redundant,
    MAX_MINUS_31..=u32::MAX => Distance::Redundant,
    HALF_MAX_PLUS_1..=MAX_MINUS_32 => Distance::Old,
    n => Distance::New(n)
  }
}

#[cfg(test)]
mod tests {
  use super::{Sequence, SentSeqNo, distance, Distance};

  #[test]
  fn test_distances() {
    // Identical sequence numbers have no distance between them
    assert_eq!(distance(0, 0), Distance::Redundant);
    assert_eq!(distance(u32::MAX, u32::MAX), Distance::Redundant);
    assert_eq!(distance(u32::MAX/2 + 7, u32::MAX/2 + 7), Distance::Redundant);
    assert_eq!(distance(u32::MAX/2 - 55, u32::MAX/2 - 55), Distance::Redundant);

    // With numbers whose difference is <= u32::MAX/2 + 1...
    // If the start sequence number is greater, the distance is old as it would be very negative
    assert_eq!(distance(1, 0), Distance::Redundant);
    assert_eq!(distance(32, 0), Distance::Redundant);
    assert_eq!(distance(33, 0), Distance::Old);

    assert_eq!(distance(9999, 495), Distance::Old);
    assert_eq!(distance(u32::MAX, u32::MAX/2), Distance::Old);
    // If the start sequence number is smaller, the distance is the difference
    assert_eq!(distance(0, 1), Distance::New(1));
    assert_eq!(distance(700, 4981), Distance::New(4281));
    assert_eq!(distance(u32::MAX/2 + 1, u32::MAX), Distance::New(u32::MAX/2));

    // With numbers whose difference is > u32::MAX/2 + 1...
    // If the start sequence number is much greater, the distance wraps around and is positive
    assert_eq!(distance(u32::MAX/2 + 2, 0), Distance::New(u32::MAX/2));
    assert_eq!(distance(u32::MAX - 9999, 495), Distance::New(10495));
    assert_eq!(distance(u32::MAX, u32::MAX/4), Distance::New(1073741824));
    // If the start sequence number is much smaller, the distance wraps around and is negative (aka None)
    assert_eq!(distance(0, u32::MAX/2 + 1), Distance::Old);
    assert_eq!(distance(700, 40_00_000_981), Distance::Old);
    assert_eq!(distance(u32::MAX/2, u32::MAX), Distance::Old);
  }

  mod update_remote {
    use super::Sequence;

    // Expect zero-gap to no-op
    #[test]
    fn test_zero_gap() {
      let mut seq = Sequence::new();
      seq.update_remote(1234, 0);
      assert_eq!(seq.remote_seq_no, 0);
      assert_eq!(seq.remote_seq_tail, 0);
    }

    #[test]
    // Gap larger than 31 erases the remote tail
    fn test_large_gap() {
      let mut seq = Sequence::new();
      seq.update_remote(1234, 32);
      assert_eq!(seq.remote_seq_no, 1234);
      assert_eq!(seq.remote_seq_tail, 0);
    }

    #[test]
    // Gap between 31 shifts the tail by gap amount
    fn test_gap() {
      let mut seq = Sequence::new();
      for n in 1..=31 {
        // Testing with a full tail
        seq.remote_seq_no = 0;
        seq.remote_seq_tail = u32::MAX;
        seq.update_remote(n, n);

        // Note the first bit after the first shift becomes a 1 (to account for the prev remote sequence no.)
        // Therefore the bitfield appears to only shift (n - 1) spaces
        assert_eq!(seq.remote_seq_no, n);
        assert_eq!(seq.remote_seq_tail, u32::MAX << (n-1));

        // Testing with an empty tail
        seq.remote_seq_no = 0;
        seq.remote_seq_tail = 0;
        seq.update_remote(n, n);

        // Note the first bit after the first shift becomes a 1 (to account for the prev remote sequence no.)
        // Therefore the bitfield appears to only shift (n - 1) spaces
        assert_eq!(seq.remote_seq_no, n);
        assert_eq!(seq.remote_seq_tail, 1 << (n-1));
      }
    }
  }

  mod iter_acks {
    use super::{Sequence, SentSeqNo};
    use crate::constants::SENT_SEQ_BUF_SIZE;
    use std::time::Instant;

    #[test]
    // With nothing in the sent buf, nothing gets acked
    fn empty_sent_buf() {
      let mut seq = Sequence::new();
      // Attempt to ack [0-32] sends
      assert_eq!(seq.iter_acks(32, u32::MAX).count(), 0);
    }

    #[test]
    fn already_acked_sent_buf() {
      let mut seq = Sequence::new();
      for n in 0..=32 {
        seq.sent_seq_buf[n] = Some(SentSeqNo { seq_no: n as u32, acked: true, when: Instant::now() });
      }

      // Attempt to ack [0-32] sends
      assert_eq!(seq.iter_acks(32, u32::MAX).count(), 0);
    }

    #[test]
    fn fully_unacked_sent_buf() {
      let mut seq = Sequence::new();
      for n in 0..=32 {
        seq.sent_seq_buf[n] = Some(SentSeqNo::new(n as u32, Instant::now()));
      }

      // Attempt to ack 33 (0..=32) sends
      assert_eq!(seq.iter_acks(32, u32::MAX).count(), 33);

      // Iterating acks them, so future calls don't re-ack
      assert_eq!(seq.iter_acks(32, u32::MAX).count(), 0);
    }

    #[test]
    fn partial_ack_sent_buf() {
      let mut seq = Sequence::new();
      for n in 0..=32 {
        if n % 2 == 0 {
          seq.sent_seq_buf[n] = Some(SentSeqNo::new(n as u32, Instant::now()));
        }
      }

      // Attempt to ack 16 (0..=15) sends
      // Only the evens qualify, for a total of 8 acks
      assert_eq!(seq.iter_acks(15, 2u32.pow(16) - 1).count(), 8);

      // Iterating acks them, so future calls don't re-ack
      assert_eq!(seq.iter_acks(15, 2u32.pow(16) - 1).count(), 0);
    }

    #[test]
    fn wrapping_partial_ack_sent_buf() {
      let mut seq = Sequence::new();
      for n in (SENT_SEQ_BUF_SIZE-16)..=(SENT_SEQ_BUF_SIZE+16) {
        if n % 2 == 0 {
          seq.sent_seq_buf[n % SENT_SEQ_BUF_SIZE] = Some(SentSeqNo::new(n as u32, Instant::now()));
        }
      }

      // Attempt to ack 24 sends
      // Only the evens qualify, for a total of 12 acks
      assert_eq!(seq.iter_acks((SENT_SEQ_BUF_SIZE + 15) as u32, 2u32.pow(24) - 1).count(), 12);

      // Iterating acks them, so future calls don't re-ack
      assert_eq!(seq.iter_acks(1032, 2u32.pow(16) - 1).count(), 0);
    }

    #[test]
    // Some entries in the buf may be stale. We do not ack them even if ackable
    fn wrapping_stale_partial_ack_sent_buf() {
      let mut seq = Sequence::new();

      // Stale seq #s 0-32
      for n in 0..=32 {
        seq.sent_seq_buf[n % SENT_SEQ_BUF_SIZE] = Some(SentSeqNo::new(n as u32, Instant::now()));
      }

      // Fresh seq #s 1024-1056, evens only
      for n in SENT_SEQ_BUF_SIZE..=(SENT_SEQ_BUF_SIZE+32) {
        if n % 2 == 0 {
          seq.sent_seq_buf[n % SENT_SEQ_BUF_SIZE] = Some(SentSeqNo::new(n as u32, Instant::now()));
        }
      }

      // Attempt to ack 16 sends
      // Only the evens qualify, for a total of 8 acks
      assert_eq!(seq.iter_acks((SENT_SEQ_BUF_SIZE + 15) as u32, 2u32.pow(16) - 1).count(), 8);

      // Note, the half of the old entries that didn't get overwritten still exist
      assert_eq!(seq.iter_acks(32, u32::MAX).count(), 16);
    }

    #[test]
    // Iteration order is oldest to newest
    fn iteration_order() {
      let mut seq = Sequence::new();

      for n in u32::MAX - 15..=u32::MAX {
        seq.sent_seq_buf[n as usize % SENT_SEQ_BUF_SIZE] = Some(SentSeqNo::new(n as u32, Instant::now()));
      }
      for n in 0..=16 {
        seq.sent_seq_buf[n % SENT_SEQ_BUF_SIZE] = Some(SentSeqNo::new(n as u32, Instant::now()));
      }

      // Attempt to ack all sends
      let acks: Vec<u32> = seq.iter_acks(16, u32::MAX).map(|s| s.seq_no).collect();
      assert_eq!(acks, vec![
        4294967280, 4294967281, 4294967282, 4294967283, 4294967284, 4294967285, 4294967286, 4294967287,
        4294967288, 4294967289, 4294967290, 4294967291, 4294967292, 4294967293, 4294967294, 4294967295,
        0         , 1         , 2         , 3         , 4         , 5         , 6         , 7         ,
        8         , 9         , 10        , 11        , 12        , 13        , 14        , 15        ,
        16
      ]);
    }
  }
}
