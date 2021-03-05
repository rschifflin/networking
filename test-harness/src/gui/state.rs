use imgui::ImString;
use std::time::{Duration, Instant};

use clock::Clock;
use crate::gui::io::{input, output};
use crate::gui::Args;

pub struct State {
  t_zero: Instant,
  pub sent: Vec<(Duration, Vec<u8>)>,
  pub received: Vec<(Duration, Vec<u8>)>,

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
      fields: input::Fields {
        send_string: ImString::with_capacity(128),
        log_string: ImString::with_capacity(4096),
        tick_amount: 1000
      },
      actions: output::Actions {
        send: false,
        tick: false,
        log: false
      }
    }
  }

  pub fn transition_ui(&mut self, args: &Args) {
    if self.actions.tick {
      args.clock.tick_ms(self.fields.tick_amount as u64);
      args.service.wake().expect("Could not wake");
    }

    if self.actions.send {
      let to_send_bytes = self.fields.send_string.to_string().as_bytes().to_vec();
      args.socket.send(&to_send_bytes).expect("Could not send");
      let send = (args.clock.now() - self.t_zero, to_send_bytes.clone());
      self.sent.push(send);

      self.fields.send_string.clear();
    }

    if self.actions.log {
      println!("{}", self.fields.log_string);
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
        }
      }
    }
  }
}

