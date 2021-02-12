use std::sync::Mutex;
use xor::Xor;

fn main() {
  let lhs = Mutex::new(false);
  let rhs = Mutex::new(false);
  let both = &(lhs, rhs);
  let mut xor = Xor::new(both);

  let mut lhs = xor.lhs().lock().unwrap();
  *lhs = true;

  let mut rhs = xor.rhs().lock().unwrap();
  *rhs = true;
}
