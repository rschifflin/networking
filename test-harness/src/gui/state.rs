use std::time::{Duration, Instant};

use clock::Clock;
use crate::gui::io::{input, Input, Output};
use crate::gui::Args;

pub struct State {
  t_zero: Instant,

  pub sent: Vec<(Duration, Vec<u8>)>,
  pub received: Vec<(Duration, Vec<u8>)>,
  pub to_send: String
}

impl State {
  pub fn new(args: &Args) -> State {
    let t_zero = args.clock.now();
    State {
      t_zero,
      sent: vec![],
      received: vec![],
      to_send: String::new(),
    }
  }

  pub fn ui_in(&self) -> Input {
    Input {
      sent: &self.sent,
      received: &self.received,

      fields: input::Fields {
        send_string: &self.to_send,
      }
    }
  }

  pub fn transition_ui(&mut self, args: &Args, ui_out: Output) {
    ui_out.fields.send_string.map(|send_string| {
      self.to_send = send_string;
    });

    if ui_out.actions.tick {
      args.clock.tick_1s();
      args.service.wake().expect("Could not wake");
    }

    if ui_out.actions.send {
      let to_send_bytes = self.to_send.as_bytes().to_vec();
      args.socket.send(&to_send_bytes).expect("Could not send");
      let send = (args.clock.now() - self.t_zero, to_send_bytes.clone());
      self.sent.push(send);
    }
  }

  pub fn transition_socket(&mut self, args: &Args, buf: &mut [u8]) {
    loop {
      match args.socket.recv(&mut buf[..]) {
        Err(_) => break,
        Ok(size) => {
          let now = args.clock.now();
          self.received.push((now - self.t_zero, buf[..size].to_vec()));
        }
      }
    }
  }
}

