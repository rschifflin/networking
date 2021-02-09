use std::sync::{Arc, Mutex};
use bring::Bring;
pub type SharedRingBuf = Arc<Mutex<Bring>>;

