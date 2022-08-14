#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(rust_2018_idioms)]
#![warn(clippy::all)]

use std::{
    env, panic,
    path::PathBuf,
    time::{Duration, Instant},
};

use egui_glium::EguiGlium;
use glium::{
    glutin::{
        self,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy},
        window::WindowBuilder,
    },
    Display, Surface,
};
use serde::{Deserialize, Serialize};

mod app;
use app::{op_queue::Op, App};
mod icon;
mod vec2;
use vec2::Vec2;
mod rect;
mod util;
use util::UserEvent;
mod image_io;

#[derive(Serialize, Deserialize, Debug, Default)]
struct Config {
    maximized: bool,
}

pub struct System {
    pub event_loop: EventLoop<UserEvent>,
    pub proxy: EventLoopProxy<UserEvent>,
    pub display: glium::Display,
    pub egui: EguiGlium,
    pub app: App,
}

impl System {
    pub fn new() -> Self {
        let config: Config = confy::load("simp").unwrap_or_default();

        let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event();
        let proxy = event_loop.create_proxy();
        let context = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_depth_buffer(0)
            .with_srgb(true)
            .with_stencil_buffer(0)
            .with_multisampling(0);
        let builder = WindowBuilder::new()
            .with_title(String::from("Simp"))
            .with_visible(false)
            .with_min_inner_size(glutin::dpi::LogicalSize::new(640f64, 400f64))
            .with_inner_size(glutin::dpi::LogicalSize::new(1100f64, 720f64))
            .with_maximized(config.maximized)
            .with_window_icon(Some(icon::get_icon()));

        let display =
            Display::new(builder, context, &event_loop).expect("Failed to initialize display");

        let app = {
            let window_context = display.gl_window();
            let window = window_context.window();

            let size = window.inner_size();

            App::new(proxy.clone(), [size.width as f32, size.height as f32])
        };

        let egui = egui_glium::EguiGlium::new(&display);

        display.gl_window().window().set_visible(true);

        let ctrl_proxy = proxy.clone();
        ctrlc::set_handler(move || {
            let _ = ctrl_proxy.send_event(UserEvent::Exit);
        })
        .unwrap();

        Self {
            event_loop,
            proxy,
            display,
            egui,
            app,
        }
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

impl System {
    pub fn main_loop(self) {
        let System {
            event_loop,
            display,
            mut egui,
            mut app,
            ..
        } = self;

        event_loop.run(move |event, _, control_flow| {
            let mut redraw = || {
                let needs_repaint = egui.run(&display, |egui_ctx| {
                    app.handle_ui(&display, egui_ctx);
                });

                let (exit, delay) = app.update(&display);
                *control_flow = if exit {
                    ControlFlow::Exit
                } else if needs_repaint {
                    display.gl_window().window().request_redraw();
                    glutin::event_loop::ControlFlow::Poll
                } else if let Some(delay) = delay {
                    ControlFlow::WaitUntil(Instant::now() + delay)
                } else {
                    ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1))
                };

                {
                    let mut target = display.draw();

                    target.clear_color_srgb(0.172, 0.172, 0.172, 1.0);

                    // draw things behind egui here
                    let (width, height) = display.get_framebuffer_dimensions();
                    let size = Vec2::new(width as f32, height as f32);

                    if let Some(image) = app.image_view.as_mut() {
                        image.render(&mut target, size);
                    }

                    egui.paint(&display, &mut target);

                    target.finish().unwrap();
                }
            };

            match event {
                Event::RedrawEventsCleared if cfg!(windows) => redraw(),
                Event::RedrawRequested(_) if !cfg!(windows) => redraw(),
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                Event::LoopDestroyed => {
                    let window_context = display.gl_window();
                    let window = window_context.window();
                    let data = Config {
                        maximized: window.is_maximized(),
                    };
                    confy::store("simp", data).unwrap();
                }
                Event::WindowEvent { event, .. } => {
                    if !egui.on_event(&event) || matches!(event, WindowEvent::MouseWheel { .. }) {
                        app.handle_window_event(&display, &event);
                    }
                    display.gl_window().window().request_redraw();
                }
                Event::UserEvent(mut event) => app.handle_user_event(&display, &mut event),
                _ => display.gl_window().window().request_redraw(),
            }
        });
    }
}

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        let _ = msgbox::create(
            "Error",
            &format!("panic occurred: {}", panic_info),
            msgbox::IconType::Error,
        );
        std::process::exit(1);
    }));

    let mut system = System::new();

    let mut args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        if let Some(arg) = args.pop() {
            system.app.queue(Op::LoadPath(PathBuf::from(arg), true))
        }
    }

    system.main_loop();
}
