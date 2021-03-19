use socket2::{Socket, Domain, Type};
use std::net::{UdpSocket, SocketAddr};
use std::sync::{Arc, Mutex};
use clock::mock;



pub struct Harness {
  pub clock: mock::Clock,
  pub socket: UdpSocket,
  pub cb_hist: Arc<Mutex<CallbackHist>>
}

pub struct CallbackHist {
  pub on_sent: Vec<(Vec<u8>, u32)>,
  pub on_acked: Vec<u32>
}

impl CallbackHist {
  pub fn new() -> CallbackHist {
    CallbackHist {
      on_sent: vec![],
      on_acked: vec![]
    }
  }
}

pub fn new(listen_port: u16, peer_port: u16) -> Harness {
  let listen_addr: SocketAddr = format!("127.0.0.1:{}", listen_port).parse().unwrap();
  let peer_addr: SocketAddr = format!("127.0.0.1:{}", peer_port).parse().unwrap();

  let socket = Socket::new(Domain::IPV4, Type::DGRAM, None).expect("Could not construct socket");
  socket.set_nonblocking(true).expect("Could not set nonblocking!");
  socket.set_reuse_address(true).expect("Could not set so_reuseaddr");
  socket.bind(&listen_addr.into()).expect("Could not bind");
  let socket: UdpSocket = socket.into();

  let clock = mock::Clock::new(std::time::Instant::now());
  let cb_hist = Arc::new(Mutex::new(CallbackHist::new()));
  let cb_hist_on_sent = cb_hist.clone();
  let cb_hist_on_acked = cb_hist.clone();

  let service = gudp::Builder::new()
    .clock(clock.clone())
    .on_packet_sent(Box::new(move |_addr_pair, buf, sequence_no| {
      let mut callbacks = cb_hist_on_sent.lock().expect("Could not acquire unpoisoned callback hist lock");
      callbacks.on_sent.push((buf.to_vec(), sequence_no));
    }))
    .on_packet_acked(Box::new(move |_addr_pair, sequence_no| {
      let mut callbacks = cb_hist_on_acked.lock().expect("Could not acquire unpoisoned callback hist lock");
      callbacks.on_acked.push(sequence_no);
    }))
    .build()
    .expect("Could not initialize gudp service");

  let listener = service.listen(socket).expect("Could not start listener");
  std::thread::spawn(move || listen(listener));

  let socket = Socket::new(Domain::IPV4, Type::DGRAM, None).expect("Could not construct socket");
  socket.set_nonblocking(true).expect("Could not set nonblocking!");
  socket.set_reuse_address(true).expect("Could not set so_reuseaddr");
  socket.bind(&peer_addr.into()).expect("Could not bind");
  let socket: UdpSocket = socket.into();
  socket.connect(&listen_addr).expect("Could not set connect");

  Harness {
    clock,
    socket,
    cb_hist
  }
}

fn listen(listener: gudp::Listener) {
  loop {
    let conn = listener.accept().expect("Could not accept connection on listener");
    std::thread::spawn(move || { on_accept(conn) });
  }
}

fn on_accept(conn: gudp::Connection) -> std::io::Result<()> {
  let mut buf = [0u8; 1000];
  loop {
    let recv_len = conn.recv(&mut buf)?;
    match std::str::from_utf8(&buf[..recv_len]) {
      Ok("ping") => { /* heartbeat */ },
      _ => {
        conn.send(&buf[..recv_len]).expect("Could not send");
      }
    }
  }
}
