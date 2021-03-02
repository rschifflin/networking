fn main() {
  let usage = "Usage: gudp-client [-l] <src port> <dst port>";
  let mut args = std::env::args().skip(1).rev();
  let dst_port_string = args.next().expect(usage);
  let dst_port = dst_port_string.parse::<u16>().expect(usage);
  let src_port_string = args.next().expect(usage);
  let src_port = src_port_string.parse::<u16>().expect(usage);

  let src_addr = format!("127.0.0.1:{}", src_port);
  let dst_addr = format!("127.0.0.1:{}", dst_port);

  let socket = std::net::UdpSocket::bind(src_addr).expect("Could not bind");
  socket.set_nonblocking(true).expect("Could not set nonblocking!");

  let service = gudp::Service::initialize().expect("Could not initialize gudp service");
  if let Some("-l") = args.next().as_deref() {
    let listener = gudp::listen(&service, socket).expect("Could not start listener");
    listen(listener, src_port);
  } else {
    let connection = gudp::connect(&service, socket, dst_addr).expect("Could not connect gudp");
    ping(connection)
  }
}

fn listen(listener: gudp::Listener, src_port: u16) {
  println!("Listening on {} for messages", src_port);
  let mut n_remaining = 3;
  let mut threads = vec![];

  while n_remaining > 0 {
    let conn = listener.accept().expect("Could not accept connection on listener");
    threads.push(std::thread::spawn(move || { on_accept(conn) }));
    n_remaining -= 1;
  }

  println!("Reached limit on accepting listener connections. Closing listener");
  drop(listener);
  for thread in threads {
    thread.join().expect("Could not join accepted connection thread").ok();
  }
}

fn on_accept(conn: gudp::Connection) -> std::io::Result<()> {
  let src_port = conn.local_addr().port();
  let dst_port = conn.peer_addr().port();
  println!("Accepted connection on {} for messages from {}", src_port, dst_port);
  let mut buf = [0u8; 1000];
  let mut heartbeats = 0;
  loop {
    let recv_len = conn.recv(&mut buf)?;
    let recv_str = std::str::from_utf8(&buf[..recv_len]).expect("Did not recv utf8");
    if recv_str == "ping" {
      heartbeats += 1;
    } else {
      println!("[From {} ({})]: {}", dst_port, heartbeats, recv_str);
      heartbeats = 0;
    }
  }
}

fn ping(conn: gudp::Connection) {
  let src_port = conn.local_addr().port();
  let dst_port = conn.peer_addr().port();
  println!("Sending stdin from {} to {}", src_port, dst_port);
  let mut buf = [0u8; 1000];
  let mut send_string = String::new();
  let stdin = std::io::stdin();

  loop {
    // TODO: add a try_recv_iter?
    loop {
      match conn.try_recv(&mut buf) {
        Some(Ok(recv_len)) => {
          if &buf[..recv_len] != b"ping" {
            println!("[From {}]: {}", dst_port, std::str::from_utf8(&buf[..recv_len]).expect("Did not recv utf8"));
          }
        },
        _ => {
          break;
        }
      }
    };

    send_string.clear();
    stdin.read_line(&mut send_string).expect("Could not read stdin");
    send_string.pop(); // To remove the newline
    conn.send(send_string.as_bytes()).expect("Failed to send");

    // Brief sleep for an optional response to arrive
    std::thread::sleep(std::time::Duration::from_millis(50));
  }
}
