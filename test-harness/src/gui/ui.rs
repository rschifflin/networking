use std::borrow::Cow;
use std::fmt::Write;

use imgui::{im_str, ImString, ListBox};
use crate::gui::io::input::fields::Current;
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

fn display_selected(ui: &imgui::Ui, current: &Current) {
  ui.label_text(&ImString::new("Time (Selected)"), &current.elapsed_string);
  ui.spacing();
  ui.spacing();
  ui.label_text(&ImString::new("Magic bytes (Selected)"), &current.magic_bytes_hexstring);
  ui.spacing();
  ui.spacing();
  ui.label_text(&ImString::new("Local sequence number (Selected)"), &current.local_sequence_no_numstring);
  ui.spacing();
  ui.spacing();
  ui.label_text(&ImString::new("Remote sequence number (Selected)"), &current.remote_sequence_no_numstring);
  ui.spacing();
  ui.spacing();
  ui.label_text(&ImString::new("Remote sequence tail (Selected)"), &current.remote_sequence_tail_bitstring);
  ui.spacing();
  ui.spacing();
  ui.label_text(&ImString::new("Payload (Selected)"), &current.payload_string);
}

pub fn populate_frame<'a>(ui: &imgui::Ui, state: &mut State) {
  imgui::Window::new(im_str!("Test harness"))
    .size([600.0, 100.0], imgui::Condition::FirstUseEver)
    .build(ui, || {
      imgui::TabBar::new(im_str!("basictabbar")).build(&ui, || {
        imgui::TabItem::new(im_str!("Home")).build(&ui, || {
          ui.text(im_str!("Header"));

          if ui.input_text(im_str!("Magic bytes"), &mut state.fields.home.magic_bytes_hexstring)
            .chars_hexadecimal(true)
            .build() {
              sanitize_hexstring(&mut state.fields.home.magic_bytes_hexstring, 8);
          }

          if ui.input_text(im_str!("Local sequence number"), &mut state.fields.home.local_sequence_no_numstring)
            .chars_decimal(true)
            .build() {
              sanitize_u32(&mut state.fields.home.local_sequence_no_numstring);
          }

          if ui.input_text(im_str!("Remote sequence number"), &mut state.fields.home.remote_sequence_no_numstring)
            .chars_decimal(true)
            .build() {
              sanitize_u32(&mut state.fields.home.remote_sequence_no_numstring);
          }

          if ui.input_text(im_str!("Remote sequence tail (Oldest to Newest) "), &mut state.fields.home.remote_sequence_tail_bitstring).build() {
            sanitize_bitstring(&mut state.fields.home.remote_sequence_tail_bitstring, 32)
          }

          ui.spacing();
          ui.spacing();

          ui.text(im_str!("Payload (Leave empty for heartbeat)"));
          ui.input_text(im_str!("Payload"), &mut state.fields.home.payload_string).build();

          ui.spacing();
          ui.spacing();

          ui.separator();
          ui.text(im_str!("Packet preview"));
          ui.text_wrapped(&mut state.fields.home.send_hexstring);
          ui.separator();
          state.actions.send = ui.button(im_str!("Send Packet"), [0.0, 0.0]);

          ui.spacing();
          ui.spacing();

          ui.text(im_str!("Time elapsed: "));
          ui.same_line(0.0);
          ui.text(&state.fields.home.elapsed_string);

          state.actions.tick = ui.button(im_str!("Tick"), [0.0, 0.0]);
          ui.same_line(0.0);
          if ui.input_int(im_str!("ms"), &mut state.fields.home.tick_amount).build() {
            state.fields.home.tick_amount = state.fields.home.tick_amount.max(0);
          }

          ui.spacing();
          ui.separator();
          ui.spacing();
          state.actions.log = ui.input_text_multiline(im_str!("Log"), &mut state.fields.home.log_string, [0.0, 60.0])
            .enter_returns_true(true)
            .build();
        });

        imgui::TabItem::new(im_str!("Sent")).build(&ui, || {
          ui.text(im_str!("SENT:"));
          if state.fields.sent.list.len() > 0 {
            state.actions.select_sent = ListBox::new(im_str!("sentlist"))
              .build_simple(ui, &mut state.fields.sent.list_idx, state.fields.sent.list.as_slice(), &|s: &ImString| Cow::Borrowed(s.as_ref()));
            display_selected(ui, &state.fields.sent.current);
          }
        });

        imgui::TabItem::new(im_str!("Received")).build(&ui, || {
          ui.text(&ImString::new("RECEIVED:"));
          if state.fields.received.list.len() > 0 {
            state.actions.select_received = ListBox::new(im_str!("receivedlist"))
              .build_simple(ui, &mut state.fields.received.list_idx, state.fields.received.list.as_slice(), &|s: &ImString| Cow::Borrowed(s.as_ref()));

            let data = &state.received[state.received.len() - state.fields.received.list_idx - 1].1;
            display_selected(ui, &state.fields.received.current);
          }
        });
      });
    });
}
