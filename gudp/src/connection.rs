use mio::Waker;
use std::sync::Arc;
use crate::types::SharedRingBuf;

// A user-facing GUDP Connection interface
pub struct Connection {
    waker: Arc<Waker>,
    buf_read: SharedRingBuf,
    buf_write: SharedRingBuf
}

impl Connection {
    pub fn new(waker: Arc<Waker>, buf_read: SharedRingBuf, buf_write: SharedRingBuf) -> Connection {
      Connection { waker, buf_read, buf_write }
    }

    pub fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
      let mut buf_write = self.buf_write.lock().expect("Could not acquire unpoisoned write lock");
      match buf_write.push_blob_back(buf) {
        Some(size) => {
          drop(buf_write);
          self.waker.wake().expect("Could not wake"); // Wake on send to flush all writes immediately
          Ok(size)
        },
        None => {
          std::io::Result::Err(
            std::io::Error::new(std::io::ErrorKind::WouldBlock, "Nothing to send")
          )
        }
      }
    }

    pub fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
      loop {
        {
          let mut buf_read = self.buf_read.lock().expect("Could not acquire unpoisoned read lock");
          if buf_read.count() > 0 {
            return buf_read.pop_blob_front(buf).map(std::io::Result::Ok).unwrap_or_else(|| {
              std::io::Result::Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "Nothing to recv"))
            });
          }
        } // Release the lock and sleep
        std::thread::sleep(std::time::Duration::from_millis(10));
      }
    }
}
