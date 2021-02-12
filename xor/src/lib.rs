mod xor;
mod r#mut;
mod owned;

pub use crate::xor::Xor;
pub use crate::r#mut::Xor as XorMut;
pub use crate::owned::Xor as XorOwned;

#[test]
fn test_compiler_disallows_accessing_both() {
  let t = trybuild::TestCases::new();
  t.compile_fail("tests/disallow/*.rs");
}

/*

#[test]
fn test_disallow_borrowing_both_mut() {
  let lhs = vec![3usize];
  let rhs = vec![5usize];
  let both = &mut (lhs, rhs);
  let mut xor = XorMut::new(both);

  let hold_left = xor.lhs();
  let hold_right = xor.rhs();
  assert_eq!(hold_left.pop(), Some(3usize));
  assert_eq!(hold_right.pop(), Some(5usize));
}

#[test]
fn test_disallow_borrowing_both_arc_mutex() {
  use std::sync::Arc;
  let lhs = vec![8usize];
  let rhs = vec![1usize];
  let both = Arc::new((lhs, rhs));
  let mut xor = Xor::new(both);

  let hold_left = xor.lhs();
  let hold_right = xor.rhs();
  assert_eq!(hold_left.pop(), Some(8usize));
  assert_eq!(hold_right.pop(), Some(1usize));
}
*/
