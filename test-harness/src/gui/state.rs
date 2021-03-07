use std::time::{Duration, Instant};
use std::fmt::Write;

use clock::Clock;
use crate::gui::io::{input, output};
use crate::gui::Args;

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
      println!("TICK {}", self.fields.tick_amount);
      args.clock.tick_ms(self.fields.tick_amount as u64);
      args.service.wake().expect("Could not wake");

      let now = args.clock.now();
      let elapsed = format!("{:?}", now - self.t_zero);
      self.fields.elapsed_string.clear();
      self.fields.elapsed_string.push_str(&elapsed);
    }

    if self.actions.send {
      args.socket.send(&self.to_send).expect("Could not send");
      let send = (args.clock.now() - self.t_zero, self.to_send.clone());
      self.sent.push(send);

      print!("SENT");
      for byte in self.to_send.iter() {
        print!(" {:02x}", byte);
      }
      print!("\n");
    }

    if self.actions.log {
      println!("LOG {}", self.fields.log_string);
      self.fields.log_string.clear();
    }
  }

  pub fn transition_socket(&mut self, args: &Args, buf: &mut [u8]) {
    loop {
      match args.socket.recv(&mut buf[..]) {
        Err(_) => break,
        Ok(size) => {
          let now = args.clock.now();
          self.received.push((now - self.t_zero, buf[..size].to_vec()));
          print!("RECEIVED");
          for byte in buf[..size].iter() {
            print!(" {:02x}", byte);
          }
          print!("\n");
        }
      }
    }
  }

  fn update_to_send(&mut self) {
    self.to_send.clear();
    let magic_bytes_hexstring = self.fields.magic_bytes_hexstring.to_string();
    self.to_send.push(u8::from_str_radix(&magic_bytes_hexstring[0..2], 16).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&magic_bytes_hexstring[2..4], 16).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&magic_bytes_hexstring[4..6], 16).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&magic_bytes_hexstring[6..8], 16).unwrap_or(0));

    let local_sequence_no_numstring = self.fields.local_sequence_no_numstring.to_string();
    let local_sequence_no = local_sequence_no_numstring.parse::<u32>().unwrap_or(0);

    // Network endian
    self.to_send.push((local_sequence_no >> 24) as u8);
    self.to_send.push(((local_sequence_no & 0b11111111_00000000_00000000) >> 16) as u8);
    self.to_send.push(((local_sequence_no & 0b11111111_00000000) >> 8) as u8);
    self.to_send.push((local_sequence_no & 0b11111111) as u8);

    let remote_sequence_no_numstring = self.fields.remote_sequence_no_numstring.to_string();
    let remote_sequence_no = remote_sequence_no_numstring.parse::<u32>().unwrap_or(0);

    // Network endian
    self.to_send.push((remote_sequence_no >> 24) as u8);
    self.to_send.push(((remote_sequence_no & 0b11111111_00000000_00000000) >> 16) as u8);
    self.to_send.push(((remote_sequence_no & 0b11111111_00000000) >> 8) as u8);
    self.to_send.push((remote_sequence_no & 0b11111111) as u8);

    // Note: index 8, 17, and 26 are whitespace
    let remote_sequence_tail_bitstring = self.fields.remote_sequence_tail_bitstring.to_string();
    self.to_send.push(u8::from_str_radix(&remote_sequence_tail_bitstring[..8], 2).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&remote_sequence_tail_bitstring[9..17], 2).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&remote_sequence_tail_bitstring[18..26], 2).unwrap_or(0));
    self.to_send.push(u8::from_str_radix(&remote_sequence_tail_bitstring[27..35], 2).unwrap_or(0));

    let payload = self.fields.payload_string.to_string();
    self.to_send.extend(payload.as_bytes());

    self.fields.send_hexstring.clear();
    for byte in self.to_send.iter() {
      write!(&mut self.fields.send_hexstring, "{:02x} ", byte);
    }
  }
}

