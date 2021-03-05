use std::net::UdpSocket;

use glium::glutin;
use glutin::event_loop::EventLoop;

use state::State;
use crate::test_clock::TestClock;

mod ui;
mod sys;
mod io;
mod state;
mod run;

pub struct Args {
  pub service: gudp::Service,
  pub socket: UdpSocket,
  pub clock: TestClock
}

pub fn gui_loop(args: Args) {
    let event_loop = EventLoop::new();
    let display = init_display(&event_loop);
    let mut ui_sys = sys::init(&display);
    let mut window_support = imgui_winit_support::WinitPlatform::init(&mut ui_sys.imgui);
    {
      let window = display.gl_window();
      let window = window.window();
      window_support.attach_window(ui_sys.imgui.io_mut(), window, imgui_winit_support::HiDpiMode::Default);
    }

    run::run(args, event_loop, ui_sys, display, window_support)
}

fn init_display(event_loop: &EventLoop<()>) -> glium::Display {
    let context = glutin::ContextBuilder::new().with_depth_buffer(24).with_vsync(false); // hard cap at 60fps
    let builder = glutin::window::WindowBuilder::new()
      .with_title("Test harness")
      .with_inner_size(glutin::dpi::LogicalSize::new(1024f64, 768f64));

    glium::Display::new(builder, context, event_loop).expect("Unable to create glium display")
}
