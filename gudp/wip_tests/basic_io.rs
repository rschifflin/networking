use std::sync::{Arc, Mutex, MutexGuard};
use std::io::{Result, ErrorKind};
use gudp::ConnectionIO;

#[derive(Clone)]
pub struct TestIO {
  inner: Arc<Mutex<_TestIO>>
}

pub struct _TestIO {
  pub n_sends: usize,
  pub sent_to: Vec<Vec<u8>>,
  pub reply_send: Vec<Result<usize>>,

  pub n_recvs: usize,
  pub recv_from: Vec<Vec<u8>>,
  pub reply_recv: Vec<Result<usize>>,
}

impl Default for _TestIO {
  fn default() -> _TestIO {
    _TestIO {
      n_sends: 0,
      sent_to: Vec::new(),
      reply_send: Vec::new(),

      n_recvs: 0,
      recv_from: Vec::new(),
      reply_recv: Vec::new()
    }
  }
}

impl Default for TestIO {
  fn default() -> TestIO {
    TestIO {
      inner: Arc::new(Mutex::new(_TestIO::default()))
    }
  }
}

impl TestIO {
  pub fn get_inner(&self) -> MutexGuard<'_, _TestIO> {
    self.inner.lock().expect("Could not get lock on TestIO")
  }
}

impl ConnectionIO for TestIO {
  fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
    let mut io = self.inner.lock().expect("Could not get lock for TestIO mutex");
    io.sent_to.push(buf.to_vec());
    io.n_sends += 1;
    io.reply_send.pop().unwrap_or_else(|| Ok(buf.len()))
  }

  fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut io = self.inner.lock().expect("Could not get lock for TestIO mutex");
    let src_buf = io.recv_from.pop().unwrap_or_else(|| vec!());
    io.n_recvs += 1;

    if src_buf.len() == 0 {
      return Err(std::io::Error::new(ErrorKind::WouldBlock, "Nothing to recv"));
    }

    let bytes_copied =
      if src_buf.len() > buf.len() {
        buf.copy_from_slice(&src_buf[..buf.len()]);
        buf.len()
      } else {
        buf[..src_buf.len()].copy_from_slice(&src_buf);
        src_buf.len()
      };

    io.reply_recv.pop().unwrap_or_else(|| Ok(bytes_copied))
  }
}
