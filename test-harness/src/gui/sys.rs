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
