use std::ops::Deref;

/// A reference to a tuple holding T and U, but preventing borrowing both at once.
pub struct Xor<T,U, Both: Deref<Target=(T,U)>> {
  pair: Both
}

impl<T,U,Both: Deref<Target=(T,U)>> Xor<T,U,Both> {
  pub fn new(pair: Both) -> Xor<T,U,Both> {
    Xor { pair }
  }

  pub fn lhs(&mut self) -> &T {
    &self.pair.0
  }

  pub fn rhs(&mut self) -> &U {
    &self.pair.1
  }
}

#[test]
fn test_ref() {
  let lhs = vec![1usize];
  let rhs = vec![2usize];
  let both = &(lhs, rhs);
  let mut xor = Xor::new(both);
  assert_eq!(xor.lhs().as_slice(), &[1usize]);
  assert_eq!(xor.rhs().as_slice(), &[2usize]);
}

#[test]
fn test_manual_drop_reallows() {
  use std::sync::Mutex;

  let lhs = Mutex::new(false);
  let rhs = Mutex::new(false);
  let both = &(lhs, rhs);
  let mut xor = Xor::new(both);

  let mut lhs = xor.lhs().lock().unwrap();
  *lhs = true;
  drop(lhs);
  let mut rhs = xor.rhs().lock().unwrap();
  *rhs = true;
}

#[test]
fn test_with_arc() {
  use std::sync::Arc;

  let lhs = vec![8usize];
  let rhs = vec![1usize];
  let both = Arc::new((lhs, rhs));
  let mut xor = Xor::new(both);
  assert_eq!(xor.lhs().first(), Some(&8usize));
  assert_eq!(xor.rhs().first(), Some(&1usize));
}
