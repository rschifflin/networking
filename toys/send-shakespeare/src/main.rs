use std::sync::Arc;
use std::sync::mpsc::{channel, Sender, Receiver, RecvTimeoutError};
use std::collections::{HashMap, HashSet};
use std::net::{UdpSocket, SocketAddr};
use std::time::Duration;
use clock::sys::Clock;
use clock::Clock as ClockT;

const WORKERS: usize = 4;
const DONE_MSG: [u8; 13] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 33, 100, 111, 110, 101]; // "(line# 0xffffff)!done"
const DONE_LINE_NO: usize = usize::MAX;

const SLEEP_RATE_MAX: f32 = 2.0;
const SLEEP_RATE_MIN: f32 = 0.1;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Message {
  Sent(usize, u32),
  Received(u32)
}

fn main() {
  env_logger::init();
  let shakespeare = include_str!("../shakespeare.txt");

  let mut addr_table = HashMap::new();
  let mut ack_txs = vec![];
  let mut ack_rxs = vec![];

  for i in (0..WORKERS) {
    let (ack_tx, ack_rx) = channel();
    ack_txs.push(ack_tx.clone());
    ack_rxs.push(Some(ack_rx));
  };

  let dst_port = 8000;
  let dst_ip_addr = "18.144.22.158";
  let dst_addr: SocketAddr = format!("{}:{}", dst_ip_addr, dst_port).parse().expect("Could not parse dst addr");

  let mut src_port = 9000;
  let workers: Vec<(usize, UdpSocket, Receiver<Message>)> = (0..WORKERS).map(|n| {
    let src_addr: SocketAddr = format!("0.0.0.0:{}", src_port).parse().expect("Could not parse src addr");
    let socket = UdpSocket::bind(&src_addr).expect("Could not bind");
    let rx = ack_rxs[n].take().expect("Could not take ack receiver");
    src_port += 1;

    socket.set_nonblocking(true).expect("Could not set nonblocking!");
    addr_table.insert((src_addr, dst_addr), n);

    (n, socket, rx)
  }).collect();

  let addr_table_acked = Arc::new(addr_table);
  let addr_table_sent = Arc::clone(&addr_table_acked);
  let ack_txs_acked = ack_txs.clone();
  let ack_txs_sent = ack_txs;
  let mut recv_counter = 0;
  let mut sent_counter = 0;
  let service = gudp::Builder::new()
    .clock(Clock())
    .on_packet_acked(Box::new(move |addr_pair, sequence_no| {
      let worker = addr_table_acked.get(&addr_pair).expect("Unknown worker");
      let sender = &ack_txs_acked[*worker];
      recv_counter += 1;
      if recv_counter % 10000 == 0 {
        println!("Recv counter checkpoint: {}", recv_counter);
      }
      sender.send(Message::Received(sequence_no));
    }))
    .on_packet_sent(Box::new(move |addr_pair, buf, sequence_no| {
      if buf.len() < 8 { return }
      let mut bytes = [0u8; 8];
      bytes.copy_from_slice(&buf[..8]);
      let line = usize::from_be_bytes(bytes);
      let worker = addr_table_sent.get(&addr_pair).expect("Unknown worker");
      let sender = &ack_txs_sent[*worker];
      sent_counter += 1;
      if sent_counter % 1000000 == 0 {
        println!("Sent counter checkpoint: {}", sent_counter);
      }
      sender.send(Message::Sent(line, sequence_no));
    }))
    .build()
    .expect("Could not initialize gudp service");


  println!("Starting with {} workers...", WORKERS);
  let time_start = Clock().now();
  let handles: Vec<std::thread::JoinHandle<isize>> = workers.into_iter().map(|(n, socket, rx)| {
    let conn = service.connect(socket, &dst_addr).expect("Could not connect");
    let handle = spawn(n, shakespeare, conn, rx);
    handle
  }).collect();

  let loss_total: isize = handles.into_iter().filter_map(|handle| handle.join().ok()).sum();
  let time_end = Clock().now();
  println!("All done. Total time: {:?}. Total re-transmits: {}", time_end - time_start, loss_total);
}

fn spawn(n: usize, shakespeare: &'static str, conn: gudp::Connection, rx: Receiver<Message>) -> std::thread::JoinHandle<isize> {
  std::thread::spawn(move || {
    println!("Connected {}", conn.local_addr());
    // Time for all the threads to initially handshake
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut sleep_rate: f32 = 2.0*SLEEP_RATE_MIN;
    let mut sleep_rate_min: f32 = SLEEP_RATE_MIN;
    let mut send_accumulator: f32 = 0.0;

    let mut sends = HashMap::new();
    let mut acks = HashSet::new();
    let mut buf = vec![0u8; 1024];
    let mut rtt_total = 0;
    let mut rtt_count = 0;
    let mut remaining = shakespeare.lines().skip(n).step_by(WORKERS).count();
    let mut loss = -1 * remaining as isize;

    while remaining > 0 {
      println!("{} Looping. Remaining: {}", n, remaining);
      loss += remaining as isize;
      for (iteration, (line_no, line)) in shakespeare.lines().enumerate().skip(n).step_by(WORKERS).enumerate() {
        if acks.contains(&line_no) { continue; }

        let line_bytes = line.as_bytes();
        buf[0..8].copy_from_slice(&line_no.to_be_bytes());
        buf[8..line_bytes.len() + 8].copy_from_slice(line_bytes);

        let rtt_sample = send_msg(&conn, &buf[..line_bytes.len() +  8], &mut send_accumulator, &mut sleep_rate, &mut sleep_rate_min);
        rtt_total += rtt_sample;
        rtt_count += 1;
        remaining -= check_acks(&mut sends, &mut acks, &rx);
      }
    }

    println!("rtt avg: {}", rtt_total / rtt_count);
    remaining = 1;
    while remaining > 0 {
      send_msg(&conn, &DONE_MSG, &mut send_accumulator, &mut sleep_rate, &mut sleep_rate_min);
      std::thread::sleep(std::time::Duration::from_millis(100));
      remaining -= check_acks(&mut sends, &mut acks, &rx);
    }

    loss
  })
}

fn send_msg(
  conn: &gudp::Connection, buf: &[u8], send_acc: &mut f32, sleep_rate: &mut f32, sleep_rate_min: &mut f32) -> u32 {
  conn.send(buf).expect("Failed to send");

  let loss = conn.loss_pct();

  if loss > 50 {
    *sleep_rate = (*sleep_rate + (SLEEP_RATE_MAX/32.0)).min(SLEEP_RATE_MAX);
  } else {
    *sleep_rate = (*sleep_rate - (SLEEP_RATE_MAX/32.0)).max(*sleep_rate_min);
  }

  *send_acc += *sleep_rate;
  let sleep_ms = send_acc.floor() as u64;
  if sleep_ms > 0 {
    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
  }
  *send_acc -= send_acc.trunc();

  conn.rtt_ms()
}

fn check_acks(sends: &mut HashMap<u32, usize>, acks: &mut HashSet<usize>, rx: &Receiver<Message>) -> usize {
  let mut acked = 0;
  for msg in rx.try_iter() {
    match msg {
      Message::Sent(line_no, seq_no) => {
        sends.insert(seq_no, line_no);
      },
      Message::Received(seq_no) => {
        sends.remove(&seq_no).map(|line_no| {
          if acks.insert(line_no) {
            acked += 1;
          }
        });
      }
    }
  }
  acked
}
