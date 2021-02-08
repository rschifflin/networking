use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use blob_ring::BlobRing;
use crate::types::SharedRingBuf;
use crate::constants::CONFIG_BUF_SIZE_BYTES;


// A trait for the IO object that underlies GUDP connections. Matches Rust's std UDP Socket api
pub trait ConnectionIO {
  fn send(&self, buf: &[u8]) -> std::io::Result<usize>;
  fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize>;
}

// A trait for the timekeeping that GUDP uses for measuring RTT
pub trait Clock {
  fn now(&self) -> std::time::Instant;
}

impl Clock for () {
  fn now(&self) -> std::time::Instant {
    std::time::Instant::now()
  }
}

// A user-facing GUDP Connection interface
pub struct Connection<IO=UdpSocket>
  where IO: ConnectionIO {
    io: IO,
    buf_read: SharedRingBuf,
    buf_write: SharedRingBuf
}

impl<IO> Connection<IO>
  where IO: ConnectionIO {
    pub fn new(io: IO, buf_read: SharedRingBuf, buf_write: SharedRingBuf) -> Connection<IO> {
      Connection { io, buf_read, buf_write }
    }

    pub fn send(&self, packet: &[u8]) -> std::io::Result<usize> {
      self.io.send(packet)
    }

    pub fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
      self.io.recv(buf)
    }

    pub fn from_io(io: IO) -> Connection<IO> {
    let buf_read_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
    let buf_write_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
    let buf_read: SharedRingBuf = Arc::new(Mutex::new(BlobRing::from_vec(buf_read_vec)));
    let buf_write: SharedRingBuf = Arc::new(Mutex::new(BlobRing::from_vec(buf_write_vec)));

      Connection {
        io,
        buf_read,
        buf_write
      }
    }

    pub fn test_send(&self, buf: &[u8]) -> std::io::Result<usize> {
      let mut buf_write = self.buf_write.lock().expect("Could not acquire unpoisoned write lock");
      buf_write.push_blob_back(buf).map(std::io::Result::Ok).unwrap_or_else(|| {
        std::io::Result::Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "Nothing to send"))
      })
    }

    pub fn test_recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
      loop {
        {
          let mut buf_read = self.buf_read.lock().expect("Could not acquire unpoisoned read lock");
          if buf_read.count() > 0 {
            return buf_read.pop_blob_front(buf).map(std::io::Result::Ok).unwrap_or_else(|| {
              std::io::Result::Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "Nothing to recv"))
            });
          }
          drop(buf_read)
        } // Release the lock and sleep

        std::thread::sleep(std::time::Duration::from_millis(1000));
      }
    }
}

impl ConnectionIO for std::net::UdpSocket {
  fn send(&self, packet: &[u8]) -> std::io::Result<usize> {
    self.send(packet)
  }

  fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
    self.recv(buf)
  }
}

pub struct Bind {
  socket: UdpSocket
}

impl Bind {
  pub fn new(port: u16) -> Bind {
    let socket = UdpSocket::bind(format!("127.0.0.1:{}", port)).expect("Could not bind to src port");
    Bind { socket }
  }

  pub fn connect(self, dest: u16) -> Connection {
    let io = self.socket;
    io.connect(format!("127.0.0.1:{}", dest)).expect("Could not connect to dest port");
    Connection::from_io(io)
  }
}
