use std::time::Instant;

use crate::constants::SENT_SEQ_BUF_SIZE;

pub type SeqNo = u32;

#[derive(Clone)]
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
      sent_seq_buf: vec![None; 1024]
    }
  }

  pub fn update_remote(&mut self, seq_no: SeqNo, seq_gap: usize) {
    // If the gap is >= 32, simply set the remote tail to 0
    // If the gap is < 32, left-shift by 1, set LSB to 1, then left-shift by GAP - 1

    match seq_gap {
      0 => return,
      1..=31 => {
        let mut tail = self.remote_seq_tail << 1;
        tail |= 1;
        self.remote_seq_tail = tail << (seq_gap - 1);
      },
      gte_32 => self.remote_seq_tail = 0
    }

    self.remote_seq_no = seq_no;
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
  type Item = SeqNo;
  fn next(&mut self) -> Option<SeqNo> {
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
              return Some(ack_seq_no);
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
                return Some(ack_seq_no);
              }
            }
          }
        }
      }
    }
  }
}

// Returns the distance from start to end if positive
// Returns None if negative or zero
// NOTE: As sequence numbers wrap around, the end magnitude may be less than the start,
// but the distance remains positive. Distance is relative to the start, up to u32::max/2 ahead.
pub fn distance(start: SeqNo, end: SeqNo) -> Option<usize> {
  let offset_end = end.wrapping_sub(start);
  if offset_end > u32::MAX/2 || offset_end == 0 {
    None
  } else {
    Some(offset_end as usize)
  }
}
