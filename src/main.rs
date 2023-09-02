#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(clippy::all)]

use std::{
    fs, panic,
    path::PathBuf,
    time::{Duration, Instant},
};

use egui_glium::EguiGlium;
use glium::{
    glutin::{
        self,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy},
        window::WindowBuilder,
    },
    Display, Surface,
};
use serde::{Deserialize, Serialize};

mod app;
use app::{
    op_queue::Op,
    preferences::{Preferences, PREFERENCES},
    App,
};
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
    preferences: Preferences,
}

pub struct WindowHandler {
    pub event_loop: EventLoop<UserEvent>,
    pub proxy: EventLoopProxy<UserEvent>,
    pub display: glium::Display,
    pub egui: EguiGlium,
    pub app: App,
}

impl WindowHandler {
    pub fn new() -> Self {
        let mut config: Config = confy::load("simp", None).unwrap_or_default();
        config.preferences.clamp();

        let event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
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

        let egui = egui_glium::EguiGlium::new(&display, &event_loop);

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

impl Default for WindowHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowHandler {
    pub fn main_loop(self) {
        let WindowHandler {
            event_loop,
            display,
            mut egui,
            mut app,
            proxy,
        } = self;

        event_loop.run(move |event, _, control_flow| {
            let mut redraw = || {
                let repaint_after = egui.run(&display, |egui_ctx| {
                    app.handle_ui(&display, egui_ctx);
                });

                let (exit, delay) = app.update(&display);
                *control_flow = if exit {
                    ControlFlow::Exit
                } else if repaint_after.is_zero() {
                    display.gl_window().window().request_redraw();
                    glutin::event_loop::ControlFlow::Poll
                } else if let Some(delay) = delay {
                    let smaller = if delay < repaint_after {
                        delay
                    } else {
                        repaint_after
                    };
                    ControlFlow::WaitUntil(Instant::now() + smaller)
                } else {
                    ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1))
                };

                {
                    let mut target = display.draw();

                    target.clear_color(0.172, 0.172, 0.172, 1.0);

                    // draw things behind egui here
                    let (width, height) = display.get_framebuffer_dimensions();
                    let size = Vec2::new(width as f32, height as f32);

                    if let Some(image) = app.image_view.as_mut() {
                        image.render(&mut target, &display, size);
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
                } => {
                    let _ = proxy.send_event(UserEvent::Exit);
                }
                Event::LoopDestroyed => {
                    let window_context = display.gl_window();
                    let window = window_context.window();
                    let data = Config {
                        maximized: window.is_maximized(),
                        preferences: PREFERENCES.lock().unwrap().clone(),
                    };
                    window.set_visible(false);
                    confy::store("simp", None, data).unwrap();
                }
                Event::WindowEvent { event, .. } => {
                    let res = egui.on_event(&event);
                    if !res.consumed || matches!(event, WindowEvent::MouseWheel { .. }) {
                        app.handle_window_event(&display, &event);
                    }

                    if res.repaint {
                        display.gl_window().window().request_redraw();
                    }
                }
                Event::UserEvent(mut event) => app.handle_user_event(&display, &mut event),
                _ => display.gl_window().window().request_redraw(),
            }
        });
    }
}

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        let dirs = directories::UserDirs::new();
        let mut path = PathBuf::from("/panic.txt");
        if let Some(dirs) = dirs {
            if let Some(desktop) = dirs.desktop_dir() {
                path = desktop.to_path_buf().join("panic.txt");
            }
        }
        eprintln!("{panic_info:?}");
        let _ = fs::write(path, format!("{panic_info:?}"));
        std::process::exit(1);
    }));

    let matches = clap::Command::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(clap::Arg::new("FILE").help("Load this file").index(1))
        .get_matches();

    let path: Option<&String> = matches.get_one("FILE");

    let mut buffer = Vec::new();
    if !atty::is(atty::Stream::Stdin) {
        use std::io::{stdin, Read};
        let _ = stdin().read_to_end(&mut buffer);
    }

    let mut window_handler = WindowHandler::new();

    if !buffer.is_empty() {
        window_handler.app.queue(Op::LoadBytes(buffer));
    } else if let Some(path) = path {
        window_handler
            .app
            .queue(Op::LoadPath(PathBuf::from(path), true))
    }

    window_handler.main_loop();
}
