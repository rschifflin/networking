use std::fmt::Write;

use imgui::{im_str, ImString};
use crate::gui::state::State;

pub fn populate_frame<'a>(ui: &imgui::Ui, state: &mut State) {
  imgui::Window::new(im_str!("Test harness"))
    .size([600.0, 100.0], imgui::Condition::FirstUseEver)
    .build(ui, || {
      imgui::TabBar::new(im_str!("basictabbar")).build(&ui, || {
        imgui::TabItem::new(im_str!("Home")).build(&ui, || {
          ui.text(im_str!("To Send "));
          ui.same_line(0.0);
          state.actions.send = ui.input_text(im_str!("Payload"), &mut state.fields.send_string)
            .enter_returns_true(true)
            .build();

          ui.spacing();
          ui.spacing();

          ui.text(im_str!("Clock "));
          ui.same_line(0.0);
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
                write!(&mut fmt_string, "{:?} ", sent.0);
                for byte in sent.1.iter() {
                  write!(&mut fmt_string, "{:x} ", byte);
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
                write!(&mut fmt_string, "{:?} ", received.0);
                for byte in received.1.iter() {
                  write!(&mut fmt_string, "{:x} ", byte);
                }
                ui.text(&fmt_string);
                fmt_string.clear();
              }
            });
        });
      });
    });
}
