#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(rust_2018_idioms)]
#![warn(clippy::all)]

use std::{env, fs, panic, path::PathBuf, time::Instant};

use glium::{
    glutin,
    glutin::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy},
        window::WindowBuilder,
    },
    Display, Surface,
};
use imgui::{Context, FontConfig, FontSource};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use serde_derive::{Deserialize, Serialize};
use util::UserEvent;
use vec2::Vec2;

mod app;
mod clipboard;
use app::App;
mod background;
use background::Background;
mod icon;

#[derive(Serialize, Deserialize, Debug)]
struct SaveData {
    width: f64,
    height: f64,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            width: 1100f64,
            height: 720f64,
        }
    }
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
        #[cfg(target_os = "windows")]
        {
            native_windows_gui::enable_visual_styles();
        }

        let save_data = get_save_data();

        let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event();
        let proxy = event_loop.create_proxy();
        let context = glutin::ContextBuilder::new().with_vsync(true);
        let builder = WindowBuilder::new()
            .with_title(String::from("Simp"))
            .with_visible(false)
            .with_min_inner_size(glutin::dpi::LogicalSize::new(640f64, 400f64))
            .with_inner_size(glutin::dpi::LogicalSize::new(
                save_data.width,
                save_data.height,
            ))
            .with_window_icon(Some(icon::get_icon()));

        let display =
            Display::new(builder, context, &event_loop).expect("Failed to initialize display");

        let app = {
            let window_context = display.gl_window();
            let window = window_context.window();

            let pos = window.outer_position().unwrap();
            let size = window.inner_size();

            App::new(
                proxy.clone(),
                [size.width as f32, size.height as f32],
                [pos.x, pos.y],
                &display,
            )
        };

        let mut imgui = Context::create();
        imgui.set_ini_filename(None);
        /*imgui.style_mut().anti_aliased_fill = false;
        imgui.style_mut().anti_aliased_lines_use_tex = false;
        imgui.style_mut().anti_aliased_lines = false;*/

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
        //let font_size = (18.0 * hidpi_factor) as f32;
        imgui.fonts().add_font(&[
            /*FontSource::TtfData {
                data: include_bytes!("../fonts/segoeui.ttf"),
                size_pixels: font_size,
                config: Some(FontConfig {
                    rasterizer_multiply: 1.0,
                    glyph_ranges: imgui::FontGlyphRanges::default(),
                    ..FontConfig::default()
                }),
            },*/
            FontSource::DefaultFontData {
                config: Some(FontConfig {
                    size_pixels: font_size,
                    ..FontConfig::default()
                }),
            },
        ]);
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        let renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");
        display.gl_window().window().set_visible(true);

        let ctrl_proxy = proxy.clone();
        ctrlc::set_handler(move || {
            let _ = ctrl_proxy.send_event(UserEvent::Exit);
        })
        .unwrap();

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
                Event::RedrawRequested(_) => {
                    let mut ui = imgui.frame();

                    let (exit, delay) = app.update(&mut ui, &display, &mut renderer, None, None);
                    if exit {
                        *control_flow = ControlFlow::Exit;
                    } else if let Some(delay) = delay {
                        *control_flow = ControlFlow::WaitUntil(Instant::now() + delay);
                    }

                    let gl_window = display.gl_window();
                    let mut target = display.draw();
                    target.clear_color_srgb(0.156, 0.156, 0.156, 1.0);

                    let dimensions = display.get_framebuffer_dimensions();
                    let size = Vec2::new(dimensions.0 as f32, dimensions.1 as f32);
                    background.render(&mut target, size, app.top_bar_size);

                    if let Some(image) = app.image_view.as_mut() {
                        image.render(&mut target, size);
                    }

                    platform.prepare_render(&ui, gl_window.window());
                    let draw_data = ui.render();
                    renderer
                        .render(&mut target, draw_data)
                        .expect("Rendering failed");

                    app.crop.render(&mut target, size);

                    target.finish().expect("Failed to swap buffers");
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                Event::LoopDestroyed => {
                    let data = SaveData {
                        width: app.size.x() as f64,
                        height: app.size.y() as f64,
                    };
                    save_data(&data);
                }
                mut event => {
                    {
                        let mut ui = imgui.frame();

                        let (exit, delay) = match &mut event {
                            Event::WindowEvent { event, .. } => {
                                app.update(&mut ui, &display, &mut renderer, Some(event), None)
                            }
                            Event::UserEvent(event) => {
                                app.update(&mut ui, &display, &mut renderer, None, Some(event))
                            }
                            _ => app.update(&mut ui, &display, &mut renderer, None, None),
                        };

                        if exit {
                            *control_flow = ControlFlow::Exit;
                        } else if let Some(delay) = delay {
                            *control_flow = ControlFlow::WaitUntil(Instant::now() + delay);
                        }
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

fn get_save_data() -> SaveData {
    if let Ok(data) = fs::read_to_string(get_data_path()) {
        if let Ok(save) = ron::from_str::<SaveData>(&data) {
            return save;
        }
    }
    SaveData::default()
}

fn save_data(save_data: &SaveData) {
    let data = ron::to_string(save_data).unwrap();
    let _ = fs::write(get_data_path(), data);
}

fn get_data_path() -> PathBuf {
    let dirs = directories::UserDirs::new().unwrap();
    let mut home = dirs.home_dir().to_path_buf();
    home.push(".simp.ron");
    home
}

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        let _ = msgbox::create(
            "Error",
            &format!("panic occurred: {}", panic_info),
            msgbox::IconType::Error,
        );
    }));

    let system = System::new();

    let mut args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        if let Some(arg) = args.pop() {
            app::load_image::load(
                system.proxy.clone(),
                arg,
                system.app.cache.clone(),
                system.app.image_loader.clone(),
            );
        }
    }

    system.main_loop();
}
