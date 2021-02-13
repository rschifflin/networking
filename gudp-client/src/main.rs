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
    let listener = gudp::listen(&service, socket).expect("Could not start listener");
    listen(listener, &mut response_buffer, src_port, dst_port);
  } else {
    let connection = gudp::connect(&service, socket).expect("Could not connect gudp");
    ping(connection, &mut response_buffer, src_port, dst_port)
  }
}

fn listen(listener: gudp::Listener, buf: &mut [u8], src_port: u16, dst_port: u16) {
  // Block until a connection has been established
  println!("Listening on {} for messages from {}", src_port, dst_port);
  let conn = listener.accept().expect("Could not accept connection");
  println!("Accepted connection on {} for messages from {}", src_port, dst_port);

  loop {
    let recv_len = conn.recv(buf).expect("Failed to recv");
    let recv_str = std::str::from_utf8(&buf[..recv_len]).expect("Did not recv utf8");
    println!("[From {}]: {}", dst_port, recv_str);

    if recv_str == "ping" {
      conn.send(b"pong").expect("Failed to send");
      println!("> pong");
    }
  }
}

fn ping(conn: gudp::Connection, buf: &mut [u8], src_port: u16, dst_port: u16) {
  println!("Sending stdin from {} to {}", src_port, dst_port);
  let mut send_string = String::new();
  let stdin = std::io::stdin();

  loop {
    if let Some(Ok(recv_len)) = conn.try_recv(buf) {
      println!("[From {}]: {}", dst_port, std::str::from_utf8(&buf[..recv_len]).expect("Did not recv utf8"));
    };

    send_string.clear();
    stdin.read_line(&mut send_string).expect("Could not read stdin");
    send_string.pop(); // To remove the newline
    conn.send(send_string.as_bytes()).expect("Failed to send");

    // Brief sleep for an optional response to arrive
    std::thread::sleep(std::time::Duration::from_millis(50));
  }
}
