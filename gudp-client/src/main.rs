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
  let mut service = gudp::Service::initialize();
  let connection = service.connect_socket(socket);

  if let Some("-l") = args.next().as_deref() {
    println!("Listening on {} for messages from {}", src_port, dst_port);
    listen(connection, &mut response_buffer)
  } else {
    println!("Pinging {} from {}", dst_port, src_port);
    ping(connection, &mut response_buffer)
  }
}

fn listen(conn: gudp::Connection, buf: &mut [u8]) {
  loop {
    let response_len = conn.recv(buf).expect("Failed to recv");
    println!("<< {}", std::str::from_utf8(&buf[..response_len]).expect("Did not recv utf8"));
    conn.send(b"pong").expect("Failed to send");
    println!(">> pong");
  }
}

fn ping(conn: gudp::Connection, buf: &mut [u8]) {
  loop {
    conn.send(b"ping").expect("Failed to send");
    println!(">> ping");
    let response_len = conn.recv(buf).expect("Failed to recv");
    println!("<< {}", std::str::from_utf8(&buf[..response_len]).expect("Did not recv utf8"));
    std::thread::sleep(std::time::Duration::from_millis(1000));
  }
}
