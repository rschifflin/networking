use std::io::Read;
use std::marker::PhantomData;
use byteorder::{BigEndian, ReadBytesExt};
use slice_pair::SlicePairMut;

pub const PREFIX_BYTES: usize = 4;

pub trait Alloc {}
#[derive(Copy,Clone,Debug)]
pub struct AllocGrow();
#[derive(Copy,Clone,Debug)]
pub struct AllocNever();
impl Alloc for AllocGrow {}
impl Alloc for AllocNever {}

// Choice of what to do with the front blob when calling with_front:
#[derive(Copy, Clone, Debug)]
pub enum WithOpt {
  Peek, // to keep it in the ring buffer
  Pop   // or to drop it
}

#[derive(Debug)]
pub struct Bring<T: Alloc> {
  pub(crate) alloc: PhantomData<T>,
  pub(crate) buffer: Vec<u8>,
  pub(crate) count: usize,
  pub(crate) remaining: usize,
  pub(crate) head_idx: usize,
  pub(crate) next_idx: usize
}

impl<T: Alloc> Bring<T> {
  pub fn from_vec(buffer: Vec<u8>) -> Bring<T> {
    let remaining = buffer.len();
    Bring {
      alloc: PhantomData,
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

  pub fn clear(&mut self) {
    self.count = 0;
    self.remaining = self.buffer.len();
    self.head_idx = 0;
    self.next_idx = 0;
  }

  pub(crate) fn does_wrap(&self) -> bool {
    if self.count == 0 { return false }
    self.head_idx >= self.next_idx
  }

  pub fn with_front<F, R>(&mut self, dst: &mut [u8], then: F) -> Option<R> where
    F: FnOnce(&mut [u8], usize) -> (R, WithOpt) {
    self.peek_front(dst).map(|(src_size_bytes, dst_size_bytes)| {
      let (res, opt) = then(dst, dst_size_bytes);
      if let WithOpt::Pop = opt {
        self.drop_front(src_size_bytes);
      }
      res
    })
  }

  fn drop_front(&mut self, src_size_bytes: usize) {
    self.count -= 1;
    self.remaining += src_size_bytes;
    self.head_idx = (self.head_idx + src_size_bytes) % self.buffer.len();
  }

  fn peek_front(&mut self, dst: &mut [u8]) -> Option<(usize, usize)> {
    if self.count <= 0 { return None; }

    // If no wrapping has occured, it's easy
    if self.head_idx < self.next_idx {
      let dst_size_bytes = self.buffer[self.head_idx..self.head_idx+PREFIX_BYTES].as_ref().read_u32::<BigEndian>().unwrap() as usize;
      let src_size_bytes = PREFIX_BYTES + dst_size_bytes;
      if dst_size_bytes > dst.len() { return None; }
      dst[..dst_size_bytes].copy_from_slice(&self.buffer[self.head_idx+PREFIX_BYTES..self.head_idx+src_size_bytes]);
      Some((src_size_bytes, dst_size_bytes))
    } else {
      // Represent our used space as a buffer wrapping from head to tail
      let (back, front) = self.buffer.split_at_mut(self.head_idx);
      let mut pair = SlicePairMut::new(front, back);
      let dst_size_bytes = pair.range(..PREFIX_BYTES).read_u32::<BigEndian>().unwrap() as usize;
      let src_size_bytes = PREFIX_BYTES + dst_size_bytes;
      if dst_size_bytes > dst.len() { return None; }
      pair.range(PREFIX_BYTES..src_size_bytes).read(dst).unwrap();
      Some((src_size_bytes, dst_size_bytes))
    }
  }

  /// Attempt to pop blob off front of ring and write it to dst. If there's no blobs left, return None. Otherwise return size of blob
  pub fn pop_front(&mut self, dst: &mut [u8]) -> Option<usize> {
    self.peek_front(dst).map(|(src_size_bytes, dst_size_bytes)| {
      self.drop_front(src_size_bytes);
      dst_size_bytes
    })
  }
}
