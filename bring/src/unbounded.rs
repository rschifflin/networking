use byteorder::{BigEndian, WriteBytesExt};
use slice_pair::SlicePairMut;

use crate::bring::Bring as BringT;
use crate::bring::AllocGrow;
use crate::bring::PREFIX_BYTES;

pub type Bring = BringT<AllocGrow>;

impl Bring {
  /// Attempt to push blob to back of ring. If there's insufficient space, allocate new space first.
  pub fn push_back(&mut self, src: &[u8]) -> usize {
    let src_size_bytes = PREFIX_BYTES + src.len();
    if src_size_bytes > self.remaining { self.grow(src_size_bytes) }
    self.remaining -= src_size_bytes;
    self.count += 1;

    // If len() - next fits everything, it's easy
    if (self.buffer.len() - self.next_idx) >= src_size_bytes {
      self.buffer[self.next_idx..self.next_idx+PREFIX_BYTES].as_mut().write_u32::<BigEndian>(src.len() as u32).unwrap();
      self.buffer[self.next_idx+PREFIX_BYTES..self.next_idx+src_size_bytes].copy_from_slice(src);
    } else {
      // Represent our remaining space as a buffer wrapping from past the end of tail to just before the start of head
      let (back, front) = self.buffer.split_at_mut(self.next_idx);
      let mut pair = SlicePairMut::new(front, back);
      pair.range(..PREFIX_BYTES).write_u32::<BigEndian>(src.len() as u32).unwrap();
      pair.range(PREFIX_BYTES..src_size_bytes).copy_from_slice(src);
    }

    // Update state accordingly
    self.next_idx = (self.next_idx + src_size_bytes) % self.buffer.len();
    src.len()
  }

  fn grow(&mut self, amount: usize) {
    // Grow the underlying vec. Then update the head and tail to be in their new positions with the same relative offset to the front and back
    let old_len = self.buffer.len();
    self.buffer.resize(old_len + amount, 0);
    self.remaining += amount;
    // If no wrapping has occured, the buffer is still valid.

    // If wrapping has occured, we need to copy [(old_len - head)...(old_len)] to [(new_len - head)...(new_len)]
    if self.does_wrap() {
      let head_span = old_len - self.head_idx;
      let new_head_idx = self.buffer.len() - head_span;
      self.buffer.copy_within(self.head_idx..old_len, new_head_idx);
      self.head_idx = new_head_idx;
    }
  }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
      let mut dst =  [0u8; 5];

      let mut ring = super::Bring::from_vec(vec![]);
      ring.push_back(&[1,2,3]);
      ring.push_back(&[4,5,6,7]);
      ring.push_back(&[8,9,10,11,12]);

      ring.pop_front(&mut dst);
      assert_eq!(dst[..3], [1,2,3]);
      ring.pop_front(&mut dst);
      assert_eq!(dst[..4], [4,5,6,7]);
      ring.pop_front(&mut dst);
      assert_eq!(dst[..5], [8,9,10,11,12]);

      ring.push_back(&[1,2,3,4,5]);
      ring.push_back(&[6,7,8,9]);
      ring.push_back(&[10,11,12]);

      ring.pop_front(&mut dst);
      assert_eq!(dst[..5], [1,2,3,4,5]);
      ring.pop_front(&mut dst);
      assert_eq!(dst[..4], [6,7,8,9]);
      ring.pop_front(&mut dst);
      assert_eq!(dst[..3], [10,11,12]);
    }

    #[test]
    fn excessive_tail_chasing() {
      // More than enough for an initial push,
      // but need to realloc after:
      let nums = vec![0u8; (4+3) + 2];

      let mut dst =  [0u8; 3];

      let mut ring = super::Bring::from_vec(nums);
      ring.push_back(&[0; 3]);
      ring.push_back(&[1; 3]);
      for i in 1..=255 {
        ring.push_back(&[i; 3]);
        ring.pop_front(&mut dst);
      }

      assert_eq!(dst[..], [253u8; 3]);
      ring.pop_front(&mut dst);
      assert_eq!(dst[..], [254u8; 3]);
      ring.pop_front(&mut dst);
      assert_eq!(dst[..], [255u8; 3]);
    }

    #[test]
    fn bounds_checks() {
      let mut dst =  [0u8; 6];

      let mut ring = super::Bring::from_vec(vec![]);
      assert_eq!(ring.pop_front(&mut dst), None);

      ring.push_back(&[1,2,3]);
      ring.push_back(&[4,5,6,7]);
      let grow_push = ring.push_back(&[8,9,10,11,12,13]);
      assert_eq!(grow_push, 6);
      let more_push = ring.push_back(&[14,15,16,17,18]);
      assert_eq!(more_push, 5);

      ring.pop_front(&mut dst);
      assert_eq!(dst[..3], [1,2,3]);
      ring.pop_front(&mut dst);
      assert_eq!(dst[..4], [4,5,6,7]);
      ring.pop_front(&mut dst);
      assert_eq!(dst[..6], [8,9,10,11,12,13]);

      ring.push_back(&[1,2,3,4,5]);
      ring.push_back(&[6,7,8,9]);
      ring.push_back(&[10,11,12]);

      ring.pop_front(&mut dst);
      assert_eq!(dst[..5], [14,15,16,17,18]);
      ring.pop_front(&mut dst);
      assert_eq!(dst[..5], [1,2,3,4,5]);
      ring.pop_front(&mut dst);
      assert_eq!(dst[..4], [6,7,8,9]);
      let ok_pop = ring.pop_front(&mut dst);
      assert_eq!(ok_pop, Some(3));

      let underflow_pop = ring.pop_front(&mut dst);
      assert_eq!(underflow_pop, None);
    }

    #[test]
    fn with_front() {
      let mut dst =  [0u8; 5];

      let mut ring = super::Bring::from_vec(vec![]);
      assert_eq!(ring.pop_front(&mut dst), None);

      ring.push_back(&[1,2,3]);
      ring.push_back(&[4,5,6,7]);
      ring.push_back(&[8,9,10,11,12]);

      let with_result = ring.with_front(&mut dst, |buf, bytes| {
        assert_eq!(buf[..bytes], [1,2,3]);
        // Setting WithOpt::Peek keeps the front-most blob on the ring buffer
        ("any return value", crate::WithOpt::Peek)
      });
      assert_eq!(with_result.unwrap(), "any return value");

      ring.with_front(&mut dst, |buf, bytes| {
        assert_eq!(buf[..bytes], [1,2,3]);
        // Setting WithOpt::Pop removes the front-most blob from the ring buffer
        ((), crate::WithOpt::Pop)
      });

      ring.with_front(&mut dst, |buf, bytes| {
        assert_eq!(buf[..bytes], [4,5,6,7]);
        ((), crate::WithOpt::Pop)
      });

      ring.with_front(&mut dst, |buf, bytes| {
        assert_eq!(buf[..bytes], [8,9,10,11,12]);
        ((), crate::WithOpt::Pop)
      });

      let with_result: Option<()> = ring.with_front(&mut dst, |_buf, _bytes| {
        panic!("Luckily this function is never called if there is nothing to peek/pop")
      });
      assert!(with_result.is_none());
      assert_eq!(ring.count(), 0);
    }
}
