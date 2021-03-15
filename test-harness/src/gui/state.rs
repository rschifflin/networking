use std::time::{Duration, Instant};
use std::fmt::Write;

use imgui::ImString;

use clock::Clock;
use crate::gui::io::{input, output};
use crate::gui::Args;

pub const WRITE_FAILED: &'static str = "Could not write formatted string";

pub struct State {
  t_zero: Instant,
  pub sent: Vec<(Duration, Vec<u8>)>,
  pub received: Vec<(Duration, Vec<u8>)>,
  pub to_send: Vec<u8>,

  pub fields: input::Fields,
  pub actions: output::Actions

}

impl State {
  pub fn new(args: &Args) -> State {
    let t_zero = args.clock.now();
    State {
      t_zero,
      sent: vec![],
      received: vec![],
      to_send: vec![],

      fields: input::Fields::default(),
      actions: output::Actions::default()
    }
  }

  pub fn transition_ui(&mut self, args: &Args) {
    self.update_to_send();

    if self.actions.tick {
      println!("TICK {}", self.fields.home.tick_amount);
      args.clock.tick_ms(self.fields.home.tick_amount as u64);
      args.service.wake().expect("Could not wake");

      let now = args.clock.now();
      let elapsed = format!("{:?}", now - self.t_zero);
      self.fields.home.elapsed_string.clear();
      self.fields.home.elapsed_string.push_str(&elapsed);
    }

    if self.actions.send {
      args.socket.send(&self.to_send).expect("Could not send");
      let send = (args.clock.now() - self.t_zero, self.to_send.clone());

      let mut fmt_string = String::new();
      write!(&mut fmt_string, "{:04}: {:?} - ", self.fields.sent.list.len() + 1, send.0)
        .expect(WRITE_FAILED);
      for byte in send.1.iter() {
        write!(&mut fmt_string, "{:02x} ", byte).expect(WRITE_FAILED);
      }
      self.fields.sent.list.insert(0, ImString::new(&fmt_string));
      self.sent.push(send);
      self.update_select_sent();
      println!("SENT {}", fmt_string);
    }

    if self.actions.log {
      println!("LOG {}", self.fields.home.log_string);
      self.fields.home.log_string.clear();
    }

    if self.actions.select_sent {
      self.update_select_sent()
    }

    if self.actions.select_received {
      self.update_select_received()
    }
  }

  pub fn transition_socket(&mut self, args: &Args, buf: &mut [u8]) {
    loop {
      match args.socket.recv(&mut buf[..]) {
        Err(_) => break,
        Ok(size) => {
          let now = args.clock.now();
          self.received.push((now - self.t_zero, buf[..size].to_vec()));

          let mut fmt_string = String::new();
          write!(&mut fmt_string, "{:04}: {:?} - ", self.fields.received.list.len(), now - self.t_zero)
            .expect(WRITE_FAILED);
          for byte in buf[..size].iter() {
            write!(&mut fmt_string, "{:02x} ", byte).expect(WRITE_FAILED);
          }
          self.fields.received.list.insert(0, ImString::new(&fmt_string));
          self.update_select_received();

          println!("RECEIVED {}", fmt_string);
        }
      }
    }
  }

  fn update_to_send(&mut self) {
    self.to_send.clear();
    let magic_bytes_hexstring = self.fields.home.magic_bytes_hexstring.to_string();
    self.to_send.push(u8::from_str_radix(&magic_bytes_hexstring[0..2], 16).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&magic_bytes_hexstring[2..4], 16).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&magic_bytes_hexstring[4..6], 16).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&magic_bytes_hexstring[6..8], 16).unwrap_or(0));

    let local_sequence_no_numstring = self.fields.home.local_sequence_no_numstring.to_string();
    let local_sequence_no = local_sequence_no_numstring.parse::<u32>().unwrap_or(0);

    // Network endian
    self.to_send.push((local_sequence_no >> 24) as u8);
    self.to_send.push(((local_sequence_no & 0b11111111_00000000_00000000) >> 16) as u8);
    self.to_send.push(((local_sequence_no & 0b11111111_00000000) >> 8) as u8);
    self.to_send.push((local_sequence_no & 0b11111111) as u8);

    let remote_sequence_no_numstring = self.fields.home.remote_sequence_no_numstring.to_string();
    let remote_sequence_no = remote_sequence_no_numstring.parse::<u32>().unwrap_or(0);

    // Network endian
    self.to_send.push((remote_sequence_no >> 24) as u8);
    self.to_send.push(((remote_sequence_no & 0b11111111_00000000_00000000) >> 16) as u8);
    self.to_send.push(((remote_sequence_no & 0b11111111_00000000) >> 8) as u8);
    self.to_send.push((remote_sequence_no & 0b11111111) as u8);

    // Note: index 8, 17, and 26 are whitespace
    let remote_sequence_tail_bitstring = self.fields.home.remote_sequence_tail_bitstring.to_string();
    self.to_send.push(u8::from_str_radix(&remote_sequence_tail_bitstring[..8], 2).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&remote_sequence_tail_bitstring[9..17], 2).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&remote_sequence_tail_bitstring[18..26], 2).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&remote_sequence_tail_bitstring[27..35], 2).unwrap_or(0));

    let payload = self.fields.home.payload_string.to_string();
    self.to_send.extend(payload.as_bytes());

    self.fields.home.send_hexstring.clear();
    for byte in self.to_send.iter() {
      write!(&mut self.fields.home.send_hexstring, "{:02x} ", byte)
        .expect(WRITE_FAILED);
    }
  }

  fn update_select_sent(&mut self) {
    let reverse_idx = self.sent.len() - self.fields.sent.list_idx  - 1;
    let (elapsed, selected) = &self.sent[reverse_idx];
    update_current(*elapsed, selected, &mut self.fields.sent.current);
  }

  fn update_select_received(&mut self) {
    let reverse_idx = self.received.len() - self.fields.received.list_idx  - 1;
    let (elapsed, selected) = &self.received[reverse_idx];
    update_current(*elapsed, selected, &mut self.fields.received.current);
  }
}

fn update_current(elapsed: Duration, selected: &[u8], current: &mut input::fields::Current) {
  let mut bytes = [0u8; 4];
  let elapsed_string = &mut current.elapsed_string;
  elapsed_string.clear();
  write!(elapsed_string, "{:?} since start", elapsed).expect(WRITE_FAILED);

  let magic_bytes = &mut current.magic_bytes_hexstring;
  magic_bytes.clear();
  for byte in &selected[0..4] {
    write!(magic_bytes, "{:02x} ", byte).expect(WRITE_FAILED);
  }

  let local_seq_no = &mut current.local_sequence_no_numstring;
  local_seq_no.clear();
  bytes.copy_from_slice(&selected[4..8]);
  write!(local_seq_no, "{}", u32::from_be_bytes(bytes)).expect(WRITE_FAILED);

  let remote_seq_no = &mut current.remote_sequence_no_numstring;
  remote_seq_no.clear();
  bytes.copy_from_slice(&selected[4..8]);
  write!(remote_seq_no, "{}", u32::from_be_bytes(bytes)).expect(WRITE_FAILED);

  let remote_seq_tail = &mut current.remote_sequence_tail_bitstring;
  remote_seq_tail.clear();
  for byte in &selected[12..16] {
    write!(remote_seq_tail, "{:08b} ", byte).expect(WRITE_FAILED);
  }

  let payload = &mut current.payload_string;
  payload.clear();
  std::str::from_utf8(&selected[16..]).map(|s| {
    if s == "" {
      payload.push_str("(Heartbeat)");
    } else {
      payload.push_str("(Utf8) ");
      payload.push_str(s);
    }
  }).unwrap_or_else(|_| {
    payload.push_str("(Non-utf8) ");
    for byte in &selected[16..] {
      write!(payload, "{:02x} ", byte).expect(WRITE_FAILED);
    }
  });
}

