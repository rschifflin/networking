use imgui::{im_str, ImString};

pub struct System {
  pub imgui: imgui::Context,
  pub renderer: imgui_glium_renderer::Renderer
}

pub fn init(display: &glium::Display) -> System {
  let mut imgui = init_imgui();
  let renderer = init_renderer(&mut imgui, display);
  System { imgui, renderer }
}

fn init_imgui() -> imgui::Context {
  imgui::Context::create()
}

fn init_renderer(imgui: &mut imgui::Context, display: &glium::Display) -> imgui_glium_renderer::Renderer {
  imgui_glium_renderer::Renderer::init(imgui, display).expect("Unable to create imgui->glium renderer")
}

#[derive(Default)]
pub struct Output {
  pub should_ping: bool,
  pub should_tick: bool
}

pub struct Input<'a> {
  pub sent: &'a Vec<Vec<u8>>,
  pub received: &'a Vec<Vec<u8>>
}

pub fn populate_frame<'a>(ui: &imgui::Ui, input: &Input<'a>) -> Output {
  let mut should_ping = false;
  let mut should_tick = false;
  imgui::Window::new(im_str!("Test harness"))
    .size([300.0, 100.0], imgui::Condition::FirstUseEver)
    .build(ui, || {
        ui.text(&ImString::new("SENT:"));
        for sent in input.sent.iter() {
          ui.text(&ImString::new(format!("{:?}", sent)));
        }

        ui.text(&ImString::new("RECEIVED:"));
        for received in input.received.iter() {
          ui.text(&ImString::new(format!("{:?}", received)));
        }

        should_ping = ui.button(im_str!("Ping"), [0.0, 0.0]);
        should_tick = ui.button(im_str!("Tick"), [0.0, 0.0]);
    });
  Output { should_ping, should_tick }
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
