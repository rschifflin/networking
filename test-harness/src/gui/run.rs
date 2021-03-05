use std::time::{Duration, Instant};
use glium::glutin;
use glium::Surface;
use glutin::event::{Event, WindowEvent}; // Platform-agnostic; uses winit here
use glutin::event_loop::{ControlFlow, EventLoop};

use crate::gui::{ui, sys, State, Args};

const NANOS_PER_FRAME: u64 = 16_666_666; // 60 fps

pub fn run(args: Args, event_loop: EventLoop<()>, mut ui_sys: sys::System, display: glium::Display, mut window_support: imgui_winit_support::WinitPlatform) {
  let mut frame_wait_deadline = Instant::now();
  let mut recv_buf = vec![0u8; 4096];
  let mut state = State::new(&args);

  event_loop.run(move |event, _, control_flow| {
    let window = display.gl_window();
    let window = window.window();
    window_support.handle_event(ui_sys.imgui.io_mut(), window, &event);
    match event {
      Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
        *control_flow = ControlFlow::Exit
      }
      Event::MainEventsCleared => {
        window_support.prepare_frame(ui_sys.imgui.io_mut(), &window).expect("Failed to prepare frame");
        let now = Instant::now();
        if now > frame_wait_deadline {
          window.request_redraw();
        }
      }
      Event::RedrawRequested(_) => {
        // Read until WouldBlock

        state.transition_socket(&args, &mut recv_buf);
        let frame = ui_sys.imgui.frame();
        ui::populate_frame(&frame, &mut state);
        state.transition_ui(&args);

        // As a side effect, we have an imgui frame in memory we can draw
        let mut target = display.draw();
        target.clear_color_and_depth((0.3,0.3,0.7,0.0), 1.0);
        let draw_data = frame.render();
        ui_sys.renderer.render(&mut target, draw_data).expect("Failed to render imgui elements");
        target.finish().expect("Unable to finish drawing and swap to surface");

        let render_finish_time = Instant::now();
        let delta = Duration::from_nanos(NANOS_PER_FRAME);
        while render_finish_time > frame_wait_deadline {
          frame_wait_deadline += delta
        }
      }
      _ => ()
    }
  });
}
