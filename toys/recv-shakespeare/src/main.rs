use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::io::Write;

use clock::sys::Clock;

fn main() {
  env_logger::init();
  let usage = "Usage: recv-shakespeare <src port> <worker count>";
  let mut args = std::env::args().skip(1);
  let listen_port_string = args.next().expect(usage);
  let listen_port = listen_port_string.parse::<u16>().expect(usage);
  let workers_string = args.next().expect(usage);
  let workers = workers_string.parse::<usize>().expect(usage);

  let (send, recv) = channel();
  let listen_addr = format!("0.0.0.0:{}", listen_port);
  let socket = std::net::UdpSocket::bind(&listen_addr).expect("Could not bind");
  socket.set_nonblocking(true).expect("Could not set nonblocking!");

  let service = gudp::Builder::new()
    .clock(Clock())
    .build()
    .expect("Could not initialize gudp service");

  let listener = service.listen(socket).expect("Could not start listener");
  {
    let send = send.clone();
    std::thread::spawn(move || listen(listener, workers, send));
  }
  drop(send);

  let mut sorted: Vec<(usize, String)> = recv.iter().collect();
  sorted.sort_unstable_by(|this, that| this.0.cmp(&that.0));
  sorted.dedup_by(|this, that| this.0 == that.0);
  let output_string = sorted.into_iter().fold(String::new(), |mut content, (_, s)| {
    content.push_str(&s);
    content.push_str("\r\n");
    content
  });
  std::fs::write("out.txt", output_string).expect("Could not write");
}

fn listen(listener: gudp::Listener, workers: usize, tx: Sender<(usize, String)>) {
  let mut handles = vec![];
  for n in 0..workers {
    let conn = listener.accept().expect("Could not accept connection on listener");
    {
      let tx = tx.clone();
      handles.push(std::thread::spawn(move || {
        let res = on_accept(conn, tx);
        println!("Finished with conn {}: {:?}", n, res);
        res
      }));
    }
  }
  drop(listener);

  for handle in handles.into_iter() {
    handle.join().expect("Thread failed");
  }
}

fn on_accept(conn: gudp::Connection, tx: Sender<(usize, String)>) -> std::io::Result<()> {
  println!("Accepted {}", conn.peer_addr());
  let mut buf = [0u8; 1024];
  let mut bytes = [0u8; 8];
  let mut counter = 0;
  loop {
    match conn.recv(&mut buf) {
      Err(e) => return Err(e),
      Ok(recv_len) => {
        if recv_len < 8 { continue }
        counter += 1;
        if counter % 10000 == 0 {
          println!("Recv checkpoint {}", counter);
        }

        if &buf[8..recv_len] == b"!done" {
          conn.send(b"");
          return Ok(());
        }

        bytes.copy_from_slice(&buf[0..8]);
        let idx = usize::from_be_bytes(bytes);
        let rest = String::from_utf8_lossy(&buf[8..recv_len]);
        tx.send((idx, rest.to_string())).expect("Could not send");

        // Ack with a heartbeat
        conn.send(b"");
      }
    }
  }
}
