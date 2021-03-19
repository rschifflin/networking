use gui::Args;

use clock::mock;
mod gui;

fn main() {
  env_logger::init();
  let listen_port = 8000;
  let listen_addr = format!("127.0.0.1:{}", listen_port);

  let peer_port = 9000;
  let peer_addr = format!("127.0.0.1:{}", peer_port);

  let socket = std::net::UdpSocket::bind(&listen_addr).expect("Could not bind");
  socket.set_nonblocking(true).expect("Could not set nonblocking!");

  let tclock = mock::Clock::new(std::time::Instant::now());

  let service = gudp::Builder::new()
    .clock(tclock.clone())
    .on_packet_sent(Box::new(|_addr_pair, buf, sequence_no| {
      println!("ON_SENT {} {:?}", sequence_no, buf);
    }))
    .on_packet_acked(Box::new(|_addr_pair, sequence_no| {
      println!("ON_ACKED {}", sequence_no);
    }))
    .build()
    .expect("Could not initialize gudp service");

  let listener = service.listen(socket).expect("Could not start listener");
  std::thread::spawn(move || listen(listener));

  let socket = std::net::UdpSocket::bind(peer_addr).expect("Could not bind");
  socket.set_nonblocking(true).expect("Could not set nonblocking!");
  socket.connect(&listen_addr).expect("Could not set connect");
  let gui_args = Args { service, socket, clock: tclock };
  gui::gui_loop(gui_args);
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
