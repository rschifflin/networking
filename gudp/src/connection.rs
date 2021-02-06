use std::net::UdpSocket;

// A trait for the IO object that underlies GUDP connections. Matches Rust's std UDP Socket api
pub trait ConnectionIO {
  fn send(&self, buf: &[u8]) -> std::io::Result<usize>;
  fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize>;
}

// A GUDP Connection
pub struct Connection<IO>
  where IO: ConnectionIO {
    io: IO
}

impl<IO> Connection<IO>
  where IO: ConnectionIO {
    pub fn send(&self, packet: &[u8]) -> std::io::Result<usize> {
      self.io.send(packet)
    }

    pub fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
      self.io.recv(buf)
    }

    pub fn from_io(io: IO) -> Connection<IO> {
      Connection { io }
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

  pub fn connect(self, dest: u16) -> Connection<UdpSocket> {
    let io = self.socket;
    io.connect(format!("127.0.0.1:{}", dest)).expect("Could not connect to dest port");
    Connection::from_io(io)
  }
}
