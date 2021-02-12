use xor::XorMut as Xor;

fn main() {
  let lhs = vec![1usize];
  let rhs = vec![2usize];
  let both = &mut (lhs, rhs);
  let mut xor = Xor::new(both);

  let hold_left = xor.lhs();
  let hold_right = xor.rhs();

  println!("Left: {:?}", hold_left.as_slice());
}
