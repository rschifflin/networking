use std::time::{Duration, Instant};
use std::net::UdpSocket;

use glium::glutin;
use glium::Surface;
use glutin::event::{Event, WindowEvent}; // Platform-agnostic; uses winit here
use glutin::event_loop::{ControlFlow, EventLoop};

use crate::test_clock::TestClock;

mod ui;

pub struct Args {
  pub service: gudp::Service,
  pub socket: UdpSocket,
  pub clock: TestClock
}

const NANOS_PER_FRAME: u64 = 16_666_666; // 60 fps

pub fn gui_loop(args: Args) {
    let event_loop = EventLoop::new();
    let display = init_display(&event_loop);
    let mut ui_sys = ui::init(&display);
    let mut window_support = imgui_winit_support::WinitPlatform::init(&mut ui_sys.imgui);
    {
      let window = display.gl_window(); let window = window.window();
      window_support.attach_window(ui_sys.imgui.io_mut(), &window, imgui_winit_support::HiDpiMode::Default);
    }
    run(args, event_loop, ui_sys, display, window_support)
}

fn init_display(event_loop: &EventLoop<()>) -> glium::Display {
    let context = glutin::ContextBuilder::new().with_depth_buffer(24).with_vsync(false); // hard cap at 60fps
    let builder = glutin::window::WindowBuilder::new()
      .with_title("Test harness")
      .with_inner_size(glutin::dpi::LogicalSize::new(1024f64, 768f64));

    glium::Display::new(builder, context, event_loop).expect("Unable to create glium display")
}

fn run(args: Args, event_loop: EventLoop<()>, mut ui_sys: ui::System, display: glium::Display, mut window_support: imgui_winit_support::WinitPlatform) {
  // Pre-loop setup- creating our buffers
  let mut frame_wait_deadline = Instant::now();
  let mut recv_buf = vec![0u8; 4096];

  let mut sent = vec![];
  let mut received = vec![];

  event_loop.run(move |event, _, control_flow| {
    let window = display.gl_window(); let window = window.window();
    window_support.handle_event(ui_sys.imgui.io_mut(), &window, &event);
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
        // Produce an in-memory representation of a frame to draw on
        loop {
          match args.socket.recv(&mut recv_buf[..]) {
            Err(_) => break,
            Ok(size) => received.push(recv_buf[..size].to_vec())
          }
        }

        let frame = ui_sys.imgui.frame();
        let ui_in = ui::Input {
          sent: &sent,
          received: &received
        };
        let ui_out = ui::populate_frame(&frame, &ui_in);
        if ui_out.should_ping {
          let to_send = b"beep";
          args.socket.send(to_send).expect("Could not send");
          sent.push(to_send.to_vec());
        }
        if ui_out.should_tick {
          args.clock.tick_1s();
          args.service.wake().expect("Could not wake");
        }

        // Now we have a fully formed frame in memory, we want to "render" it to vertex buffers
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
