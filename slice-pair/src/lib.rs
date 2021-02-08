use std::ops::RangeBounds;
use std::ops::Bound;
use std::io::{Read, Write};

pub struct SlicePair<'a, T> {
  slice_front: &'a [T],
  slice_back: &'a [T]
}

pub struct SlicePairMut<'a, T> {
  slice_front: &'a mut [T],
  slice_back: &'a mut [T]
}

impl<'a, T> SlicePair<'a, T> {
  pub fn new(slice_front: &'a [T], slice_back: &'a [T]) -> SlicePair<'a, T> {
    SlicePair { slice_front, slice_back }
  }

  pub fn len(&self) -> usize {
    self.slice_front.len() + self.slice_back.len()
  }

  pub fn range<R: RangeBounds<usize>>(&self, bounds: R) -> SlicePair<T> {
    match (bounds.start_bound(), bounds.end_bound()) {
      (Bound::Unbounded, Bound::Unbounded) => SlicePair::new(self.slice_front, self.slice_back),
      (Bound::Unbounded, Bound::Included(n)) => self.half_open_range_idx(0, n+1),
      (Bound::Unbounded, Bound::Excluded(n)) => self.half_open_range_idx(0, *n),

      (Bound::Included(n), Bound::Unbounded) => self.half_open_range_idx(*n, self.len()),
      (Bound::Included(n), Bound::Included(m)) => self.half_open_range_idx(*n, m+1),
      (Bound::Included(n), Bound::Excluded(m)) => self.half_open_range_idx(*n, *m),

      /* Can the lower bound even be excluded?? */
      (Bound::Excluded(n), Bound::Unbounded) => self.half_open_range_idx(n+1, self.len()),
      (Bound::Excluded(n), Bound::Included(m)) => self.half_open_range_idx(n+1, m+1),
      (Bound::Excluded(n), Bound::Excluded(m)) => self.half_open_range_idx(n+1, *m)
    }
  }

  fn half_open_range_idx(&self, lower_bound_in: usize, upper_bound_ex: usize) -> SlicePair<T> {
    assert!(upper_bound_ex <= self.len(), "range end index {} out of range for slice pair of length {}", upper_bound_ex, self.len());
    assert!(lower_bound_in <= upper_bound_ex, "slice index starts at {} but ends at {}", lower_bound_in, upper_bound_ex);
    if lower_bound_in == upper_bound_ex { return SlicePair::new(&self.slice_front[..0], &self.slice_back[..0]) }

    // Above assertions guarantee lower_bound_in < upper_bound_ex <= self.len()
    let front_len = self.slice_front.len();
    if lower_bound_in < front_len {
      if upper_bound_ex <= front_len {
        SlicePair::new(&self.slice_front[lower_bound_in..upper_bound_ex], &self.slice_back[..0])
      } else {
        let back_upper_bound_ex = upper_bound_ex - front_len;
        SlicePair::new(&self.slice_front[lower_bound_in..], &self.slice_back[..back_upper_bound_ex])
      }
    } else {
      let back_lower_bound_in = lower_bound_in - front_len;
      let back_upper_bound_ex = upper_bound_ex - front_len;

      SlicePair::new(&self.slice_front[..0], &self.slice_back[back_lower_bound_in..back_upper_bound_ex])
    }
  }
}

impl<'a, T: Clone> SlicePair<'a, T> {
  pub fn to_vec(&self) -> Vec<T> {
    let mut v = self.slice_front.to_vec();
    v.extend_from_slice(self.slice_back);
    v
  }
}

impl<'a, T> SlicePairMut<'a, T> {
  pub fn new(slice_front: &'a mut [T], slice_back: &'a mut [T]) -> SlicePairMut<'a, T> {
    SlicePairMut { slice_front, slice_back }
  }

  pub fn len(&self) -> usize {
    self.slice_front.len() + self.slice_back.len()
  }

  pub fn range<R: RangeBounds<usize>>(&mut self, bounds: R) -> SlicePairMut<T> {
    match (bounds.start_bound(), bounds.end_bound()) {
      (Bound::Unbounded, Bound::Unbounded) => SlicePairMut::new(self.slice_front, self.slice_back),
      (Bound::Unbounded, Bound::Included(n)) => self.half_open_range_idx(0, n+1),
      (Bound::Unbounded, Bound::Excluded(n)) => self.half_open_range_idx(0, *n),

      (Bound::Included(n), Bound::Unbounded) => self.half_open_range_idx(*n, self.len()),
      (Bound::Included(n), Bound::Included(m)) => self.half_open_range_idx(*n, m+1),
      (Bound::Included(n), Bound::Excluded(m)) => self.half_open_range_idx(*n, *m),

      /* Can the lower bound even be excluded?? */
      (Bound::Excluded(n), Bound::Unbounded) => self.half_open_range_idx(n+1, self.len()),
      (Bound::Excluded(n), Bound::Included(m)) => self.half_open_range_idx(n+1, m+1),
      (Bound::Excluded(n), Bound::Excluded(m)) => self.half_open_range_idx(n+1, *m)
    }
  }

  fn half_open_range_idx(&mut self, lower_bound_in: usize, upper_bound_ex: usize) -> SlicePairMut<T> {
    assert!(upper_bound_ex <= self.len(), "range end index {} out of range for slice pair of length {}", upper_bound_ex, self.len());
    assert!(lower_bound_in <= upper_bound_ex, "slice index starts at {} but ends at {}", lower_bound_in, upper_bound_ex);
    if lower_bound_in == upper_bound_ex { return SlicePairMut::new(&mut self.slice_front[..0], &mut self.slice_back[..0]) }

    // Above assertions guarantee lower_bound_in < upper_bound_ex <= self.len()
    let front_len = self.slice_front.len();
    if lower_bound_in < front_len {
      if upper_bound_ex <= front_len {
        SlicePairMut::new(&mut self.slice_front[lower_bound_in..upper_bound_ex], &mut self.slice_back[..0])
      } else {
        let back_upper_bound_ex = upper_bound_ex - front_len;
        SlicePairMut::new(&mut self.slice_front[lower_bound_in..], &mut self.slice_back[..back_upper_bound_ex])
      }
    } else {
      let back_lower_bound_in = lower_bound_in - front_len;
      let back_upper_bound_ex = upper_bound_ex - front_len;

      SlicePairMut::new(&mut self.slice_front[..0], &mut self.slice_back[back_lower_bound_in..back_upper_bound_ex])
    }
  }
}

impl<'a, T: Clone> SlicePairMut<'a, T> {
  pub fn to_vec(&self) -> Vec<T> {
    let mut v = self.slice_front.to_vec();
    v.extend_from_slice(self.slice_back);
    v
  }

  pub fn clone_from_slice(&mut self, src: &[T]) {
    assert_eq!(self.len(), src.len(), "source slice length ({}) does not match destination slice length ({})", src.len(), self.len());

    let front_len = self.slice_front.len();
    self.slice_front.clone_from_slice(&src[..front_len]);
    self.slice_back.clone_from_slice(&src[front_len..self.len()]);
  }
}

impl<'a, T: Copy> SlicePairMut<'a, T> {
  pub fn copy_from_slice(&mut self, src: &[T]) {
    assert_eq!(self.len(), src.len(), "source slice length ({}) does not match destination slice length ({})", src.len(), self.len());

    let front_len = self.slice_front.len();
    self.slice_front.copy_from_slice(&src[..front_len]);
    self.slice_back.copy_from_slice(&src[front_len..self.len()]);
  }
}

impl Read for SlicePair<'_, u8> {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    if self.len() == 0 { return std::io::Result::Ok(0); }

    let front_len = self.slice_front.len();
    if buf.len() <= front_len {
      self.slice_front.read(buf)
    } else {
      let end_len = buf.len().min(self.len());
      self.slice_front.read(&mut buf[..front_len]).and_then(|front_size| {
        self.slice_back.read(&mut buf[front_len..end_len]).map(|back_size| {
          front_size + back_size
        })
      })
    }
  }
}

impl Read for SlicePairMut<'_, u8> {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    if self.len() == 0 { return std::io::Result::Ok(0); }

    let front_len = self.slice_front.len();
    if buf.len() <= front_len {
      self.slice_front.as_ref().read(buf)
    } else {
      let end_len = buf.len().min(self.len());
      self.slice_front.as_ref().read(&mut buf[..front_len]).and_then(|front_size| {
        self.slice_back.as_ref().read(&mut buf[front_len..end_len]).map(|back_size| {
          front_size + back_size
        })
      })
    }
  }
}

impl Write for SlicePairMut<'_, u8> {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    if self.len() == 0 { return std::io::Result::Ok(0); }

    let front_len = self.slice_front.len();
    if buf.len() <= front_len {
      self.slice_front.write(buf)
    } else {
      let end_len = buf.len().min(self.len());
      self.slice_front.write(&buf[..front_len]).and_then(|front_size| {
        self.slice_back.write(&buf[front_len..end_len]).map(|back_size| {
          front_size + back_size
        })
      })
    }
  }

  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}


#[cfg(test)]
mod tests {
  mod len_tests {
    #[test]
    fn zero_len_is_zero() {
      let pair: crate::SlicePair<()> = crate::SlicePair::new(&[], &[]);
      let mpair: crate::SlicePairMut<()> = crate::SlicePairMut::new(&mut [], &mut []);
      assert_eq!(pair.len(), 0);
      assert_eq!(mpair.len(), 0);
    }

    #[test]
    fn first_buf_only_len_matches() {
      let mut nums = [1,2,3,4];
      let pair: crate::SlicePair<usize> = crate::SlicePair::new(&nums, &[]);
      assert_eq!(pair.len(), 4);

      let mpair: crate::SlicePairMut<usize> = crate::SlicePairMut::new(&mut nums, &mut []);
      assert_eq!(mpair.len(), 4);
    }

    #[test]
    fn second_buf_only_len_matches() {
      let mut nums = [1,2,3,4,5,6];
      let pair: crate::SlicePair<usize> = crate::SlicePair::new(&[], &nums);
      assert_eq!(pair.len(), 6);

      let mpair: crate::SlicePairMut<usize> = crate::SlicePairMut::new(&mut [], &mut nums);
      assert_eq!(mpair.len(), 6);
    }

    #[test]
    fn both_bufs_summed_match() {
      let mut front_nums = [1,2,3,4,5,6];
      let mut back_nums = [7,8,9];
      let pair: crate::SlicePair<usize> = crate::SlicePair::new(&front_nums, &back_nums);
      assert_eq!(pair.len(), 9);

      let mpair: crate::SlicePairMut<usize> = crate::SlicePairMut::new(&mut front_nums, &mut back_nums);
      assert_eq!(mpair.len(), 9);
    }
  }

  mod read_tests {
    use std::io::Read;

    #[test]
    fn read_nothing() {
      let mut dst: [u8; 5] = [0,0,0,0,0];
      let mut pair: crate::SlicePair<u8> = crate::SlicePair::new(&[], &[]);
      let result = pair.read(&mut dst);
      assert_eq!(result.unwrap(), 0);
      assert_eq!(dst, [0,0,0,0,0]);

      let mut mpair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(&mut [], &mut []);
      let result = mpair.read(&mut dst);
      assert_eq!(result.unwrap(), 0);
      assert_eq!(dst, [0,0,0,0,0]);
    }

    #[test]
    fn read_fits_in_front() {
      let nums: [u8; 10] = [1,2,3,4,5,6,7,8,9,10];
      let mut mnums: [u8; 10] = [1,2,3,4,5,6,7,8,9,10];
      let mut dst1: [u8; 5] = [0,0,0,0,0];
      let mut dst2: [u8; 5] = [0,0,0,0,0];

      let (front, back) = nums.split_at(8);
      let mut pair: crate::SlicePair<u8> = crate::SlicePair::new(front, back);
      let result = pair.read(&mut dst1);
      assert_eq!(result.unwrap(), 5);
      assert_eq!(dst1, [1,2,3,4,5]);

      let (front, back) = mnums.split_at_mut(8);
      let mut mpair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let result = mpair.read(&mut dst2);
      assert_eq!(result.unwrap(), 5);
      assert_eq!(dst2, [1,2,3,4,5]);
    }

    #[test]
    fn read_fits_in_both() {
      let nums: [u8; 10] = [1,2,3,4,5,6,7,8,9,10];
      let mut mnums: [u8; 10] = [1,2,3,4,5,6,7,8,9,10];
      let mut dst1: [u8; 5] = [0,0,0,0,0];
      let mut dst2: [u8; 5] = [0,0,0,0,0];

      let (front, back) = nums.split_at(3);
      let mut pair: crate::SlicePair<u8> = crate::SlicePair::new(front, back);
      let result = pair.read(&mut dst1);
      assert_eq!(result.unwrap(), 5);
      assert_eq!(dst1, [1,2,3,4,5]);

      let (front, back) = mnums.split_at_mut(3);
      let mut mpair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let result = mpair.read(&mut dst2);
      assert_eq!(result.unwrap(), 5);
      assert_eq!(dst2, [1,2,3,4,5]);
    }

    #[test]
    fn read_exact() {
      let nums: [u8; 10] = [1,2,3,4,5,6,7,8,9,10];
      let mut mnums: [u8; 10] = [1,2,3,4,5,6,7,8,9,10];
      let mut dst1 = [0u8; 10];
      let mut dst2 = [0u8; 10];

      let (front, back) = nums.split_at(5);
      let mut pair: crate::SlicePair<u8> = crate::SlicePair::new(front, back);
      let result = pair.read(&mut dst1);
      assert_eq!(result.unwrap(), 10);
      assert_eq!(dst1, [1,2,3,4,5,6,7,8,9,10]);

      let (front, back) = mnums.split_at_mut(5);
      let mut mpair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let result = mpair.read(&mut dst2);
      assert_eq!(result.unwrap(), 10);
      assert_eq!(dst2, [1,2,3,4,5,6,7,8,9,10]);
    }

    #[test]
    fn read_too_big() {
      let nums: [u8; 10] = [1,2,3,4,5,6,7,8,9,10];
      let mut mnums: [u8; 10] = [1,2,3,4,5,6,7,8,9,10];
      let mut dst1 = [0u8; 11];
      let mut dst2 = [0u8; 11];

      let (front, back) = nums.split_at(5);
      let mut pair: crate::SlicePair<u8> = crate::SlicePair::new(front, back);
      let result = pair.read(&mut dst1);
      assert_eq!(result.unwrap(), 10);
      assert_eq!(dst1[..10], [1,2,3,4,5,6,7,8,9,10]);
      assert_eq!(dst1[10], 0);

      let (front, back) = mnums.split_at_mut(5);
      let mut mpair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let result = mpair.read(&mut dst2);
      assert_eq!(result.unwrap(), 10);
      assert_eq!(dst2[..10], [1,2,3,4,5,6,7,8,9,10]);
      assert_eq!(dst2[10], 0);
    }
  }

  mod write_tests {
    use std::io::Write;

    #[test]
    fn write_nothing() {
      let mut src: [u8; 5] = [1,2,3,4,5];
      let mut mpair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(&mut [], &mut []);
      let result = mpair.write(&mut src);
      assert_eq!(result.unwrap(), 0);
      assert_eq!(mpair.len(), 0);
    }

    #[test]
    fn write_fits_in_front() {
      let mut mnums = [0u8; 10];
      let src: [u8; 5] = [1,2,3,4,5];

      let (front, back) = mnums.split_at_mut(8);
      let mut mpair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let result = mpair.write(&src);
      assert_eq!(result.unwrap(), 5);
      assert_eq!(mnums, [1,2,3,4,5,0,0,0,0,0]);
    }

    #[test]
    fn write_fits_in_both() {
      let mut mnums = [0u8; 10];
      let src: [u8; 5] = [1,2,3,4,5];

      let (front, back) = mnums.split_at_mut(3);
      let mut mpair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let result = mpair.write(&src);
      assert_eq!(result.unwrap(), 5);
      assert_eq!(mnums, [1,2,3,4,5,0,0,0,0,0]);
    }

    #[test]
    fn write_exact() {
      let mut mnums = [0u8; 10];
      let src: [u8; 10] = [1,2,3,4,5,6,7,8,9,10];

      let (front, back) = mnums.split_at_mut(5);
      let mut mpair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let result = mpair.write(&src);
      assert_eq!(result.unwrap(), 10);
      assert_eq!(mnums, src);
    }

    #[test]
    fn write_too_big() {
      let mut mnums = [0u8; 10];
      let src: [u8; 11] = [1,2,3,4,5,6,7,8,9,10,11];

      let (front, back) = mnums.split_at_mut(5);
      let mut mpair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let result = mpair.write(&src);
      assert_eq!(result.unwrap(), 10);
      assert_eq!(mnums, src[..10]);
    }
  }

  mod range_tests {
    #[test]
    fn full_range() {
      let nums: [u8; 10] = [0,1,2,3,4,5,6,7,8,9];
      let (front, back) = nums.split_at(5);
      let pair: crate::SlicePair<u8> = crate::SlicePair::new(front, back);
      let sub_pair = pair.range(..);
      assert_eq!(pair.to_vec(), sub_pair.to_vec());

      let mut nums: [u8; 10] = [0,1,2,3,4,5,6,7,8,9];
      let (front, back) = nums.split_at_mut(5);
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let pair_vec = pair.to_vec();
      let sub_pair = pair.range(..);
      assert_eq!(pair_vec, sub_pair.to_vec());
    }

    #[test]
    fn empty_range() {
      let nums: [u8; 10] = [0,1,2,3,4,5,6,7,8,9];
      let (front, back) = nums.split_at(5);
      let pair: crate::SlicePair<u8> = crate::SlicePair::new(front, back);
      let sub_pair = pair.range(..0);
      assert_eq!(sub_pair.to_vec(), vec![]);

      let mut nums: [u8; 10] = [0,1,2,3,4,5,6,7,8,9];
      let (front, back) = nums.split_at_mut(5);
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let sub_pair = pair.range(..0);
      assert_eq!(sub_pair.to_vec(), vec![]);
    }

    #[test]
    fn range_all_fits_in_front() {
      let nums: [u8; 10] = [0,1,2,3,4,5,6,7,8,9];
      let (front, back) = nums.split_at(5);
      let pair: crate::SlicePair<u8> = crate::SlicePair::new(front, back);
      let sub_pair = pair.range(1..5);
      assert_eq!(sub_pair.to_vec(), vec![1,2,3,4]);

      let mut nums: [u8; 10] = [0,1,2,3,4,5,6,7,8,9];
      let (front, back) = nums.split_at_mut(5);
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let sub_pair = pair.range(1..5);
      assert_eq!(sub_pair.to_vec(), vec![1,2,3,4]);
    }

    #[test]
    fn range_spans_front_and_back() {
      let nums: [u8; 10] = [0,1,2,3,4,5,6,7,8,9];
      let (front, back) = nums.split_at(5);
      let pair: crate::SlicePair<u8> = crate::SlicePair::new(front, back);
      let sub_pair = pair.range(3..8);
      assert_eq!(sub_pair.to_vec(), vec![3,4,5,6,7]);

      let mut nums: [u8; 10] = [0,1,2,3,4,5,6,7,8,9];
      let (front, back) = nums.split_at_mut(5);
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let sub_pair = pair.range(3..8);
      assert_eq!(sub_pair.to_vec(), vec![3,4,5,6,7]);
    }

    #[test]
    fn range_all_fits_in_back() {
      let nums: [u8; 10] = [0,1,2,3,4,5,6,7,8,9];
      let (front, back) = nums.split_at(5);
      let pair: crate::SlicePair<u8> = crate::SlicePair::new(front, back);
      let sub_pair = pair.range(5..10);
      assert_eq!(sub_pair.to_vec(), vec![5,6,7,8,9]);

      let mut nums: [u8; 10] = [0,1,2,3,4,5,6,7,8,9];
      let (front, back) = nums.split_at_mut(5);
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      let sub_pair = pair.range(5..10);
      assert_eq!(sub_pair.to_vec(), vec![5,6,7,8,9]);
    }
  }

  mod copy_tests {
    #[test]
    fn empty_copy() {
      let src = [0u8; 0];
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(&mut [], &mut []);
      pair.copy_from_slice(&src);
      assert_eq!(pair.to_vec(), []);
    }

    #[test]
    fn copy_fits_with_front_empty() {
      let mut nums = [0u8; 10];
      let src = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
      let (front, back) = nums.split_at_mut(0);
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      pair.copy_from_slice(&src);
      assert_eq!(pair.to_vec(), src);
    }

    #[test]
    fn copy_fits_with_back_empty() {
      let mut nums = [0u8; 10];
      let src = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
      let (front, back) = nums.split_at_mut(10);
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      pair.copy_from_slice(&src);
      assert_eq!(pair.to_vec(), src);
    }

    #[test]
    fn copy_fits_across_span() {
      let mut nums = [0u8; 10];
      let src = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
      let (front, back) = nums.split_at_mut(5);
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      pair.copy_from_slice(&src);
      assert_eq!(pair.to_vec(), src);
    }
  }

  mod clone_tests {
    #[test]
    fn empty_clone() {
      let src = [0u8; 0];
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(&mut [], &mut []);
      pair.clone_from_slice(&src);
      assert_eq!(pair.to_vec(), []);
    }

    #[test]
    fn clone_fits_with_front_empty() {
      let mut nums = [0u8; 10];
      let src = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
      let (front, back) = nums.split_at_mut(0);
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      pair.clone_from_slice(&src);
      assert_eq!(pair.to_vec(), src);
    }

    #[test]
    fn clone_fits_with_back_empty() {
      let mut nums = [0u8; 10];
      let src = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
      let (front, back) = nums.split_at_mut(10);
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      pair.clone_from_slice(&src);
      assert_eq!(pair.to_vec(), src);
    }

    #[test]
    fn clone_fits_across_span() {
      let mut nums = [0u8; 10];
      let src = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
      let (front, back) = nums.split_at_mut(5);
      let mut pair: crate::SlicePairMut<u8> = crate::SlicePairMut::new(front, back);
      pair.clone_from_slice(&src);
      assert_eq!(pair.to_vec(), src);
    }
  }
}
