use std::ops::DerefMut;

/// A mutable reference to a tuple holding T and U, but preventing borrowing both at once.
pub struct Xor<T,U, Both: DerefMut<Target=(T,U)>> {
  pair: Both
}

impl<T,U,Both: DerefMut<Target=(T,U)>> Xor<T,U,Both> {
  pub fn new(pair: Both) -> Xor<T,U,Both> {
    Xor { pair }
  }

  pub fn lhs(&mut self) -> &mut T {
    &mut self.pair.0
  }

  pub fn rhs(&mut self) -> &mut U {
    &mut self.pair.1
  }
}

#[test]
fn test_mut() {
  let lhs = vec![3usize];
  let rhs = vec![5usize];
  let both = &mut (lhs, rhs);
  let mut xor = Xor::new(both);
  assert_eq!(xor.lhs().pop(), Some(3usize));
  assert_eq!(xor.rhs().pop(), Some(5usize));
}
