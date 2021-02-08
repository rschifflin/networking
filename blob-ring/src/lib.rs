use std::io::Read;
use slice_pair::SlicePairMut;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

const PREFIX_BYTES: usize = 4;

#[derive(Debug)]
pub struct BlobRing {
  buffer: Vec<u8>,
  count: usize,
  remaining: usize,
  head_idx: usize,
  next_idx: usize
}

impl BlobRing {
  pub fn from_vec(buffer: Vec<u8>) -> BlobRing {
    let remaining = buffer.len();
    BlobRing {
      buffer,
      count: 0,
      remaining,
      head_idx: 0,
      next_idx: 0
    }
  }

  pub fn count(&self) -> usize {
    self.count
  }

  /// Attempt to push blob to back of ring. If there's insufficient space, return None. Otherwise return size of blob
  pub fn push_blob_back(&mut self, src: &[u8]) -> Option<usize> {
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
      let (back, front) = self.buffer.split_at_mut(self.head_idx);
      let mut pair = SlicePairMut::new(front, back);
      pair.range(..PREFIX_BYTES).write_u32::<BigEndian>(src.len() as u32).unwrap();
      pair.range(PREFIX_BYTES..src_size_bytes).copy_from_slice(src);
    }

    // Update state accordingly
    self.next_idx = (self.next_idx + src_size_bytes) % self.buffer.len();
    Some(src.len())
  }

  /// Attempt to pop blob off front of ring and write it to dst. If there's no blobs left, return None. Otherwise return size of blob
  pub fn pop_blob_front(&mut self, dst: &mut [u8]) -> Option<usize> {
    if self.count <= 0 { return None; }
    self.count -= 1;

    // If no wrapping has occured, it's easy
    let (src_size_bytes, dst_size_bytes) =
      if self.head_idx < self.next_idx {
        let dst_size_bytes = self.buffer[self.head_idx..self.head_idx+PREFIX_BYTES].as_ref().read_u32::<BigEndian>().unwrap() as usize;
        let src_size_bytes = PREFIX_BYTES + dst_size_bytes;
        if dst_size_bytes > dst.len() { return None; }
        dst.copy_from_slice(&self.buffer[self.head_idx+PREFIX_BYTES..self.head_idx+src_size_bytes]);
        (src_size_bytes, dst_size_bytes)
      } else {
        // Represent our used space as a buffer wrapping from head to tail
        let (back, front) = self.buffer.split_at_mut(self.head_idx);
        let mut pair = SlicePairMut::new(front, back);
        let dst_size_bytes = pair.range(..PREFIX_BYTES).read_u32::<BigEndian>().unwrap() as usize;
        let src_size_bytes = PREFIX_BYTES + dst_size_bytes;
        if dst_size_bytes > dst.len() { return None; }
        pair.range(PREFIX_BYTES..src_size_bytes).read(dst).unwrap();
        (src_size_bytes, dst_size_bytes)
      };

    // Update state accordingly
    self.head_idx = (self.head_idx + src_size_bytes) % self.buffer.len();
    self.remaining += src_size_bytes;
    Some(dst_size_bytes)
  }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
      let nums = vec![0u8; 4+3 + 4+4 + 4+5];
      let mut dst =  [0u8; 5];

      let mut blob = crate::BlobRing::from_vec(nums);
      blob.push_blob_back(&[1,2,3]);
      blob.push_blob_back(&[4,5,6,7]);
      blob.push_blob_back(&[8,9,10,11,12]);

      blob.pop_blob_front(&mut dst);
      assert_eq!(dst[..3], [1,2,3]);
      blob.pop_blob_front(&mut dst);
      assert_eq!(dst[..4], [4,5,6,7]);
      blob.pop_blob_front(&mut dst);
      assert_eq!(dst[..5], [8,9,10,11,12]);

      blob.push_blob_back(&[1,2,3,4,5]);
      blob.push_blob_back(&[6,7,8,9]);
      blob.push_blob_back(&[10,11,12]);

      blob.pop_blob_front(&mut dst);
      assert_eq!(dst[..5], [1,2,3,4,5]);
      blob.pop_blob_front(&mut dst);
      assert_eq!(dst[..4], [6,7,8,9]);
      blob.pop_blob_front(&mut dst);
      assert_eq!(dst[..3], [10,11,12]);
    }

    #[test]
    fn excessive_tail_chasing() {
      let nums = vec![0u8; 4+10 + 7];
      let mut dst =  [0u8; 10];

      let mut blob = crate::BlobRing::from_vec(nums);
      for _ in 0..50 {
        blob.push_blob_back(&[0,1,2,3,4,5,6,7,8,9]);
        blob.pop_blob_front(&mut dst);
      }
    }

    #[test]
    fn bounds_checks() {
      let nums = vec![0u8; 4+3 + 4+4 + 4+5];
      let mut dst =  [0u8; 5];

      let mut blob = crate::BlobRing::from_vec(nums);
      assert_eq!(blob.pop_blob_front(&mut dst), None);

      blob.push_blob_back(&[1,2,3]);
      blob.push_blob_back(&[4,5,6,7]);
      let overflow_push = blob.push_blob_back(&[8,9,10,11,12,13]);
      assert_eq!(overflow_push, None);
      let ok_push = blob.push_blob_back(&[8,9,10,11,12]);
      assert_eq!(ok_push, Some(5));

      blob.pop_blob_front(&mut dst);
      assert_eq!(dst[..3], [1,2,3]);
      blob.pop_blob_front(&mut dst);
      assert_eq!(dst[..4], [4,5,6,7]);
      blob.pop_blob_front(&mut dst);
      assert_eq!(dst[..5], [8,9,10,11,12]);

      blob.push_blob_back(&[1,2,3,4,5]);
      blob.push_blob_back(&[6,7,8,9]);
      blob.push_blob_back(&[10,11,12]);

      blob.pop_blob_front(&mut dst);
      assert_eq!(dst[..5], [1,2,3,4,5]);
      blob.pop_blob_front(&mut dst);
      assert_eq!(dst[..4], [6,7,8,9]);
      let ok_pop = blob.pop_blob_front(&mut dst);
      assert_eq!(ok_pop, Some(3));

      let underflow_pop = blob.pop_blob_front(&mut dst);
      assert_eq!(underflow_pop, None);
    }
}
