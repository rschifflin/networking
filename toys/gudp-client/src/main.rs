//ex listener: RUST_BACKTRACE=1 RUST_LOG=TRACE cargo run -- -l 8000 9000 2> err.txt
//ex client: RUST_BACKTRACE=1 RUST_LOG=TRACE cargo run -- 9000 8000 2> err.txt
fn main() {
  env_logger::init();
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

  let service = gudp::Builder::new().build().expect("Could not initialize gudp service");
  if let Some("-l") = args.next().as_deref() {
    let listener = service.listen(socket).expect("Could not start listener");
    listen(listener, src_port);
  } else {
    let connection = service.connect(socket, dst_addr).expect("Could not connect gudp");
    ping(connection)
  }
}

fn listen(listener: gudp::Listener, src_port: u16) {
  println!("Listening on {} for messages", src_port);
  let mut n_remaining: Option<usize> = None;
  let mut threads = vec![];

  loop {
    match n_remaining {
      // If we eventually run out of connections, break and join all threads before exiting
      Some(n) if n <= 0 => break,
      Some(ref mut n) => {
        let conn = listener.accept().expect("Could not accept connection on listener");
        threads.push(std::thread::spawn(move || { on_accept(conn) }));
        *n -= 1;
      },

      // If we handle unlimited connections, simply loop forever and discard join handles
      None => {
        let conn = listener.accept().expect("Could not accept connection on listener");
        std::thread::spawn(move || { on_accept(conn) });
      }
    }
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
  let reader = conn.clone();
  let src_port = conn.local_addr().port();
  let dst_port = conn.peer_addr().port();
  println!("Sending stdin from {} to {}", src_port, dst_port);
  let mut buf = [0u8; 1000];
  let stdin = std::io::stdin();
  let tzero = std::time::Instant::now();
  std::thread::spawn(move || loop {
    match reader.recv(&mut buf) {
      Ok(recv_len) => {
        let now = std::time::Instant::now();
        if &buf[..recv_len] != b"ping" {
          println!("[From {} <{:?}>]: {}", dst_port, now.duration_since(tzero), std::str::from_utf8(&buf[..recv_len]).expect("Did not recv utf8"));
        }
      },
      _ => {
        break;
      }
    };
  });

  let mut n = 0;
  //let mut send_string = String::new();
  loop {
    /*
    send_string.clear();
    stdin.read_line(&mut send_string).expect("Could not read stdin");
    send_string.pop(); // To remove the newline
    conn.send(send_string.as_bytes()).expect("Failed to send");
    */
    n += 1;
    conn.send(n.to_string().as_bytes()).expect("Failed to send");

    let now = std::time::Instant::now();
    //println!("[To {} <{:?}>]: {}", dst_port, now.duration_since(tzero), send_string);
    println!("[To {} <{:?}>]: {}", dst_port, now.duration_since(tzero), n);

    std::thread::sleep(std::time::Duration::from_millis(50));
  }
}
