use std::sync::{Arc, Mutex};
use blob_ring::BlobRing;
pub type SharedRingBuf = Arc<Mutex<BlobRing>>;

