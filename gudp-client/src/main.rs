fn main() {
  let usage = "Usage: gudp-client [-l] <src port> <dst port>";
  let mut args = std::env::args().skip(1).rev();
  let dst_port_string = args.next().expect(usage);
  let dst_port = dst_port_string.parse::<u16>().expect(usage);
  let src_port_string = args.next().expect(usage);
  let src_port = src_port_string.parse::<u16>().expect(usage);

  let socket = std::net::UdpSocket::bind(format!("127.0.0.1:{}", src_port)).expect("Could not bind");
  socket.connect(format!("127.0.0.1:{}", dst_port)).expect("Could not connect");
  socket.set_nonblocking(true).expect("Could not set nonblocking!");

  let mut response_buffer = [0u8; 1000];
  let service = gudp::Service::initialize();
  if let Some("-l") = args.next().as_deref() {
    let listener = gudp::listen(&service, socket);
    listen(listener, &mut response_buffer, src_port, dst_port)
  } else {
    let connection = gudp::connect(&service, socket).expect("Could not connect gudp");
    ping(connection, &mut response_buffer, src_port, dst_port)
  }
}

fn listen(listener: gudp::Listener, buf: &mut [u8], src_port: u16, dst_port: u16) {
  // Block until a connection has been established
  let conn = listener.accept().expect("Could not accept connection");

  println!("Listening on {} for messages from {}", src_port, dst_port);
  loop {
    let response_len = conn.recv(buf).expect("Failed to recv");
    println!("[From {}]: {}", dst_port, std::str::from_utf8(&buf[..response_len]).expect("Did not recv utf8"));
    conn.send(b"pong").expect("Failed to send");
    println!("> pong");
  }
}

fn ping(conn: gudp::Connection, buf: &mut [u8], src_port: u16, dst_port: u16) {
  println!("Pinging {} from {}", dst_port, src_port);
  loop {
    conn.send(b"ping").expect("Failed to send");
    println!("> ping");
    let response_len = conn.recv(buf).expect("Failed to recv");
    println!("[From {}]: {}", dst_port, std::str::from_utf8(&buf[..response_len]).expect("Did not recv utf8"));
    std::thread::sleep(std::time::Duration::from_millis(1000));
  }
}
