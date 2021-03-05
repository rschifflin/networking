use std::fmt::Write;

use imgui::{im_str, ImString};

use crate::gui::io::{output, Input, Output};

pub fn populate_frame<'a>(ui: &imgui::Ui, input: &Input<'a>) -> Output {
  let mut send_field = ImString::with_capacity(4096);
  send_field.push_str(input.fields.send_string);

  let mut send_string = None;
  let mut should_send = false;
  let mut should_tick = false;

  imgui::Window::new(im_str!("Test harness"))
    .size([600.0, 100.0], imgui::Condition::FirstUseEver)
    .build(ui, || {
      imgui::TabBar::new(im_str!("basictabbar")).build(&ui, || {
        imgui::TabItem::new(im_str!("Home")).build(&ui, || {
          if ui.input_text(im_str!("beep"), &mut send_field).build() {
            send_string = Some(send_field.to_string());
          }

          should_send = ui.button(im_str!("Send"), [0.0, 0.0]);
          should_tick = ui.button(im_str!("Tick"), [0.0, 0.0]);
        });
        imgui::TabItem::new(im_str!("History")).build(&ui, || {
          imgui::ChildWindow::new("historychildsend").border(true)
            .size([ui.window_content_region_width() * 0.5, 0.0])
            .build(&ui, || {
              ui.text(&ImString::new("SENT:"));
              let mut fmt_string = String::new();
              for sent in input.sent.iter().rev() {
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
              for received in input.received.iter().rev() {
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

  Output {
    actions: output::Actions { send: should_send, tick: should_tick },
    fields: output::Fields { send_string }
  }
}

  /*
  let mut reset = false;
  let mut rotate = (0.0f32, 0.0, 0.0);
  let mut select_model_file_output = file_browser::Output::default();
  let mut animation_output  = animation_widget::Output::default();

  let window = imgui::Window::new(im_str!("Quaternion Camera"));
  imgui::Window::new(im_str!("Model"))
      .size([300.0, 100.0], imgui::Condition::FirstUseEver)
      .build(ui, || {
          if ui.button(im_str!("Select file"), [0.0, 0.0]) {
              ui.open_popup(im_str!("File browser"));
          }
          select_model_file_output = file_browser::build(ui, input.select_model_file);

          if input.animation_input.anim_list.len() > 0 {
            animation_output = animation_widget::build(ui, image_font, input.animation_input)
          }
      });
  window
      .size([300.0, 100.0], imgui::Condition::FirstUseEver)
      .build(ui, || {
        imgui::AngleSlider::new(im_str!("Rotate X"))
          .min_degrees(-3.0)
          .max_degrees(3.0)
          .build(&ui, &mut rotate.0);

          imgui::AngleSlider::new(im_str!("Rotate Y"))
            .min_degrees(-3.0)
            .max_degrees(3.0)
            .build(&ui, &mut rotate.1);
          imgui::AngleSlider::new(im_str!("Rotate Z"))
            .min_degrees(-3.0)
            .max_degrees(3.0)
            .build(&ui, &mut rotate.2);
          if ui.button(im_str!("Reset"), [100.0, 20.0]) {
            reset = true
          }
          ui.text(&ImString::new(&input.camera_info.0));
          ui.text(&ImString::new(&input.camera_info.1));
          ui.text(&ImString::new(&input.camera_info.2));
      });

  Output {
    rotation_euler: rotate,
    reset: reset,
    select_model_file: select_model_file_output,
    animation_output: animation_output
  }
  */
