#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use glium::{
    glutin,
    glutin::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy},
        window::WindowBuilder,
    },
    {Display, Surface},
};
use image::{ImageBuffer, Rgba};
use imgui::{Context, FontConfig, FontGlyphRanges, FontSource};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::{env, time::Instant};

mod app;
mod clipboard;
use app::App;
mod background;
use background::Background;
mod vec2;
use vec2::Vec2;
mod icon;

pub enum UserEvent {
    ImageLoaded(ImageBuffer<Rgba<u16>, Vec<u16>>),
}

pub struct System {
    pub event_loop: EventLoop<UserEvent>,
    pub proxy: EventLoopProxy<UserEvent>,
    pub display: glium::Display,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    pub font_size: f32,
    pub app: App,
    pub background: Background,
}

impl System {
    pub fn new() -> Self {
        let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event();
        let proxy = event_loop.create_proxy();
        let context = glutin::ContextBuilder::new().with_vsync(true);
        let builder = WindowBuilder::new()
            .with_title(String::from("simp"))
            .with_visible(false)
            .with_min_inner_size(glutin::dpi::LogicalSize::new(640f64, 400f64))
            .with_inner_size(glutin::dpi::LogicalSize::new(1100f64, 720f64))
            .with_window_icon(Some(icon::get_icon()));
        let display =
            Display::new(builder, context, &event_loop).expect("Failed to initialize display");

        let app = App::new(proxy.clone(), [1100f32, 720f32]);

        let mut imgui = Context::create();
        imgui.set_ini_filename(None);

        if let Some(backend) = clipboard::init() {
            imgui.set_clipboard_backend(Box::new(backend));
        } else {
            eprintln!("Failed to initialize clipboard");
        }

        let mut platform = WinitPlatform::init(&mut imgui);
        {
            let gl_window = display.gl_window();
            let window = gl_window.window();
            platform.attach_window(imgui.io_mut(), window, HiDpiMode::Rounded);
        }

        let hidpi_factor = platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.fonts().add_font(&[
            FontSource::DefaultFontData {
                config: Some(FontConfig {
                    size_pixels: font_size,
                    ..FontConfig::default()
                }),
            },
            FontSource::TtfData {
                data: include_bytes!("../fonts/mplus-1p-regular.ttf"),
                size_pixels: font_size,
                config: Some(FontConfig {
                    rasterizer_multiply: 1.75,
                    glyph_ranges: FontGlyphRanges::default(),
                    ..FontConfig::default()
                }),
            },
        ]);

        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        let renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");
        display.gl_window().window().set_visible(true);

        let background = Background::new(&display);

        Self {
            event_loop,
            proxy,
            display,
            imgui,
            platform,
            renderer,
            font_size,
            app,
            background,
        }
    }
}

//https://stackoverflow.com/questions/56701736/how-to-correctly-translate-mouse-coords-to-opengl-coords
fn _cord_to_gl(x: f32, y: f32, width: f32, height: f32) -> (f32, f32) {
    let x = 2.0 * (x / width) - 1.0;
    let y = 2.0 * ((y - height + 1.0) / height) - 1.0;
    (x, y)
}

impl System {
    pub fn main_loop(self) {
        let System {
            event_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
            mut app,
            background,
            ..
        } = self;
        let mut last_frame = Instant::now();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::NewEvents(_) => {
                    let now = Instant::now();
                    imgui.io_mut().update_delta_time(now - last_frame);
                    last_frame = now;
                }
                Event::MainEventsCleared => (),
                Event::RedrawRequested(_) => {
                    let mut ui = imgui.frame();

                    app.update(&mut ui, &display, &mut renderer, None, None);

                    let gl_window = display.gl_window();
                    let mut target = display.draw();
                    target.clear_color_srgb(0.262, 0.286, 0.337, 1.0);

                    let dimensions = display.get_framebuffer_dimensions();
                    let size = Vec2::new(dimensions.0 as f32, dimensions.1 as f32);
                    background.render(&mut target, size);

                    platform.prepare_render(&ui, gl_window.window());
                    let draw_data = ui.render();
                    renderer
                        .render(&mut target, draw_data)
                        .expect("Rendering failed");
                    target.finish().expect("Failed to swap buffers");
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                event => {
                    {
                        let mut ui = imgui.frame();

                        match &event {
                            Event::WindowEvent { event, .. } => {
                                app.update(&mut ui, &display, &mut renderer, Some(event), None)
                            }
                            Event::UserEvent(event) => {
                                app.update(&mut ui, &display, &mut renderer, None, Some(event))
                            }
                            _ => app.update(&mut ui, &display, &mut renderer, None, None),
                        };
                    }

                    let gl_window = display.gl_window();
                    platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
                    platform
                        .prepare_frame(imgui.io_mut(), gl_window.window())
                        .expect("Failed to prepare frame");
                    gl_window.window().request_redraw();
                }
            }
        });
    }
}

fn main() {
    let system = System::new();

    let mut args: Vec<String> = env::args().collect();
    if let Some(arg) = args.pop() {
        app::load_image(system.proxy.clone(), arg);
    }

    system.main_loop();
}
