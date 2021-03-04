fn main() {
  let usage = "Usage: gudp-chaingun <src port start> <range n> <dst port>";
  let mut args = std::env::args().skip(1).rev();
  let dst_port_string = args.next().expect(usage);
  let dst_port = dst_port_string.parse::<u16>().expect(usage);
  let range_len_string = args.next().expect(usage);
  let range_len = range_len_string.parse::<u16>().expect(usage);
  let range_start_string = args.next().expect(usage);
  let range_start = range_start_string.parse::<u16>().expect(usage);

  let dst_addr = format!("127.0.0.1:{}", dst_port);
  let service = gudp::Service::initialize().expect("Could not initialize gudp service");
  let mut threads = vec![];

  for src_port in range_start..(range_start + range_len) {
    let src_addr = format!("127.0.0.1:{}", src_port);
    let socket = std::net::UdpSocket::bind(src_addr).expect("Could not bind");
    socket.set_nonblocking(true).expect("Could not set nonblocking!");
    let conn = service.connect(socket, &dst_addr).expect("Could not connect gudp");
    threads.push(std::thread::spawn(|| fire(conn)));
    std::thread::sleep(std::time::Duration::from_millis(10));
  }

  for thread in threads {
    thread.join().expect("Could not join accepted connection thread");
  }
}

fn fire(conn: gudp::Connection) {
  // Long sleep (still sending heartbeats) before life
  std::thread::sleep(std::time::Duration::from_millis(5_000));

  conn.send(b"Testing").expect("Failed to send");
  std::thread::sleep(std::time::Duration::from_millis(50));
  conn.send(b"1").expect("Failed to send");
  std::thread::sleep(std::time::Duration::from_millis(1_000));
  conn.send(b"2").expect("Failed to send");
  std::thread::sleep(std::time::Duration::from_millis(1_000));
  conn.send(b"3").expect("Failed to send");

  // Long sleep (still sending heartbeats) before death
  std::thread::sleep(std::time::Duration::from_millis(5_000));
}
