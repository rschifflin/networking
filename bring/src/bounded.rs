use byteorder::{BigEndian, WriteBytesExt};
use slice_pair::SlicePairMut;

use crate::bring::Bring as BringT;
use crate::bring::AllocNever;
use crate::bring::PREFIX_BYTES;

pub type Bring = BringT<AllocNever>;

impl Bring {
  /// Attempt to push blob to back of ring. If there's insufficient space, return None. Otherwise return size of blob
  pub fn push_back(&mut self, src: &[u8]) -> Option<usize> {
    let src_size_bytes = PREFIX_BYTES + src.len();
    if src_size_bytes > self.remaining { return None; }
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
    Some(src.len())
  }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
      let nums = vec![0u8; 4+3 + 4+4 + 4+5];
      let mut dst =  [0u8; 5];

      let mut ring = super::Bring::from_vec(nums);
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
      // Enough for 3 pushes plus a small amount of extra
      let nums = vec![0u8; (4+3)*3 + 2];
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
      let nums = vec![0u8; 4+3 + 4+4 + 4+5];
      let mut dst =  [0u8; 5];

      let mut ring = super::Bring::from_vec(nums);
      assert_eq!(ring.pop_front(&mut dst), None);

      ring.push_back(&[1,2,3]);
      ring.push_back(&[4,5,6,7]);
      let overflow_push = ring.push_back(&[8,9,10,11,12,13]);
      assert_eq!(overflow_push, None);
      let ok_push = ring.push_back(&[8,9,10,11,12]);
      assert_eq!(ok_push, Some(5));

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
      let ok_pop = ring.pop_front(&mut dst);
      assert_eq!(ok_pop, Some(3));

      let underflow_pop = ring.pop_front(&mut dst);
      assert_eq!(underflow_pop, None);
    }

    #[test]
    fn front_with() {
      let nums = vec![0u8; 4+3 + 4+4 + 4+5];
      let mut dst =  [0u8; 5];

      let mut ring = super::Bring::from_vec(nums);
      assert_eq!(ring.pop_front(&mut dst), None);

      ring.push_back(&[1,2,3]);
      ring.push_back(&[4,5,6,7]);
      ring.push_back(&[8,9,10,11,12]);

      let with_result = ring.front(&mut dst).unwrap().with(|bytes| {
        assert_eq!(dst[..bytes], [1,2,3]);
        // Setting WithOpt::Peek keeps the front-most blob on the ring buffer
        ("any return value", crate::WithOpt::Peek)
      });
      assert_eq!(with_result, "any return value");

      ring.front(&mut dst).unwrap().with(|bytes| {
        assert_eq!(dst[..bytes], [1,2,3]);
        // Setting WithOpt::Pop removes the front-most blob from the ring buffer
        ((), crate::WithOpt::Pop)
      });

      ring.front(&mut dst).unwrap().with(|bytes| {
        assert_eq!(dst[..bytes], [4,5,6,7]);
        ((), crate::WithOpt::Pop)
      });

      ring.front(&mut dst).unwrap().with(|bytes| {
        assert_eq!(dst[..bytes], [8,9,10,11,12]);
        ((), crate::WithOpt::Pop)
      });

      let with_result = ring.front(&mut dst);
      assert!(with_result.is_none());
      assert_eq!(ring.count(), 0);
    }
}
