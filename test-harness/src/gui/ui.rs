use std::fmt::Write;

use imgui::{im_str, ImString};
use crate::gui::state::State;

fn sanitize_u32(numstring: &mut ImString) {
  let mut s = numstring.to_string();
  match i64::from_str_radix(&s, 10) {
    Ok(n) => {
      if (n >= 0) && (n <= u32::MAX as i64) { return }
      let clamped = n.max(0).min(u32::MAX as i64);
      numstring.clear();
      numstring.push_str(&format!("{}", clamped));
    },
    Err(_) => {
      numstring.clear();
      numstring.push_str(&format!("{}", u32::MAX));
    }
  }
}

fn sanitize_hexstring(hexstring: &mut ImString, bytes: usize) {
  let mut s = hexstring.to_string();
  hexstring.clear();
  for c in s.chars().chain(std::iter::repeat('0')).take(bytes) {
    hexstring.push(c);
  }
}

fn sanitize_bitstring(bitstring: &mut ImString, bits: usize) {
  let mut s = bitstring.to_string();
  bitstring.clear();

  s.chars()
    .filter(|c| *c == '0' || *c == '1')
    .chain(std::iter::repeat('0'))
    .take(bits)
    .fold(0, |n, c| {
      if (n % 8 == 0) && (n > 0) {
        bitstring.push(' ');
      }
      bitstring.push(c);
      n+1
    });
}

pub fn populate_frame<'a>(ui: &imgui::Ui, state: &mut State) {
  imgui::Window::new(im_str!("Test harness"))
    .size([600.0, 100.0], imgui::Condition::FirstUseEver)
    .build(ui, || {
      imgui::TabBar::new(im_str!("basictabbar")).build(&ui, || {
        imgui::TabItem::new(im_str!("Home")).build(&ui, || {
          ui.text(im_str!("Header"));

          if ui.input_text(im_str!("Magic bytes"), &mut state.fields.magic_bytes_hexstring)
            .chars_hexadecimal(true)
            .build() {
              sanitize_hexstring(&mut state.fields.magic_bytes_hexstring, 8);
          }

          if ui.input_text(im_str!("Local sequence number"), &mut state.fields.local_sequence_no_numstring)
            .chars_decimal(true)
            .build() {
              sanitize_u32(&mut state.fields.local_sequence_no_numstring);
          }

          if ui.input_text(im_str!("Remote sequence number"), &mut state.fields.remote_sequence_no_numstring)
            .chars_decimal(true)
            .build() {
              sanitize_u32(&mut state.fields.remote_sequence_no_numstring);
          }

          if ui.input_text(im_str!("Remote sequence tail (Newest <-> Oldest) "), &mut state.fields.remote_sequence_tail_bitstring).build() {
            sanitize_bitstring(&mut state.fields.remote_sequence_tail_bitstring, 32)
          }

          ui.spacing();
          ui.spacing();

          ui.text(im_str!("Payload (Leave empty for heartbeat)"));
          ui.input_text(im_str!("Payload"), &mut state.fields.payload_string).build();

          ui.spacing();
          ui.spacing();

          ui.separator();
          ui.text(im_str!("Packet preview"));
          ui.text_wrapped(&mut state.fields.send_hexstring);
          ui.separator();
          state.actions.send = ui.button(im_str!("Send Packet"), [0.0, 0.0]);

          ui.spacing();
          ui.spacing();

          ui.text(im_str!("Time elapsed: "));
          ui.same_line(0.0);
          ui.text(&state.fields.elapsed_string);

          state.actions.tick = ui.button(im_str!("Tick"), [0.0, 0.0]);
          ui.same_line(0.0);
          if ui.input_int(im_str!("ms"), &mut state.fields.tick_amount).build() {
            state.fields.tick_amount = state.fields.tick_amount.max(0);
          }

          ui.spacing();
          ui.separator();
          ui.spacing();
          state.actions.log = ui.input_text_multiline(im_str!("Log"), &mut state.fields.log_string, [0.0, 60.0])
            .enter_returns_true(true)
            .build();
        });

        imgui::TabItem::new(im_str!("History")).build(&ui, || {
          imgui::ChildWindow::new("historychildsend").border(true)
            .size([ui.window_content_region_width() * 0.5, 0.0])
            .build(&ui, || {
              ui.text(&ImString::new("SENT:"));
              let mut fmt_string = String::new();
              for sent in state.sent.iter().rev() {
                write!(&mut fmt_string, "{:?} - ", sent.0);
                for byte in sent.1.iter() {
                  write!(&mut fmt_string, "{:02x} ", byte);
                }
                ui.text(&fmt_string);
                fmt_string.clear();
              }
            });
          ui.same_line(0.0);
          imgui::ChildWindow::new("historychildrecv").border(true)
            .size([ui.window_content_region_width() * 0.5, 0.0])
            .build(&ui, || {
              ui.text(&ImString::new("RECEIVED:"));
              let mut fmt_string = String::new();
              for received in state.received.iter().rev() {
                write!(&mut fmt_string, "{:?} - ", received.0);
                for byte in received.1.iter() {
                  write!(&mut fmt_string, "{:02x} ", byte);
                }
                ui.text(&fmt_string);
                fmt_string.clear();
              }
            });
        });
      });
    });
}
