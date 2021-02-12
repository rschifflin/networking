/// A tuple holding T and U, but preventing borrowing both at once.
pub struct Xor<T,U> {
  pair: (T,U)
}

impl<T,U> Xor<T,U> {
  pub fn new(pair: (T,U)) -> Xor<T,U> {
    Xor { pair }
  }

  pub fn lhs(&mut self) -> &T {
    &self.pair.0
  }

  pub fn rhs(&mut self) -> &U {
    &self.pair.1
  }

  pub fn lhs_mut(&mut self) -> &mut T {
    &mut self.pair.0
  }

  pub fn rhs_mut(&mut self) -> &mut U {
    &mut self.pair.1
  }
}

#[test]
fn test_owned() {
  let lhs = vec![1usize];
  let rhs = vec![2usize];
  let both = (lhs, rhs);
  let mut xor = Xor::new(both);
  assert_eq!(xor.lhs().as_slice(), &[1usize]);
  assert_eq!(xor.rhs().as_slice(), &[2usize]);
}

#[test]
fn test_owned_mut() {
  let lhs = vec![8usize];
  let rhs = vec![1usize];
  let both = (lhs, rhs);
  let mut xor = Xor::new(both);
  assert_eq!(xor.lhs_mut().pop(), Some(8usize));
  assert_eq!(xor.rhs_mut().pop(), Some(1usize));
}
