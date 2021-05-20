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
use imgui::{Context, FontConfig, FontGlyphRanges, FontSource, Ui};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::time::Instant;

mod clipboard;

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
}

pub fn init() -> System {
    let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event();
    let proxy = event_loop.create_proxy();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let builder = WindowBuilder::new()
        .with_title(String::from("simp"))
        .with_visible(false)
        .with_min_inner_size(glutin::dpi::LogicalSize::new(640f64, 400f64))
        .with_inner_size(glutin::dpi::LogicalSize::new(1100f64, 720f64));
    let display =
        Display::new(builder, context, &event_loop).expect("Failed to initialize display");

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
            data: include_bytes!("../../fonts/mplus-1p-regular.ttf"),
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

    System {
        event_loop,
        proxy,
        display,
        imgui,
        platform,
        renderer,
        font_size,
    }
}

impl System {
    pub fn main_loop<
        F: FnMut(
                &mut bool,
                &mut Ui,
                &glium::Display,
                &mut Renderer,
                Option<&WindowEvent>,
                Option<&UserEvent>,
            ) + 'static,
    >(
        self,
        mut run_ui: F,
    ) {
        let System {
            event_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
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

                    let mut run = true;
                    run_ui(&mut run, &mut ui, &display, &mut renderer, None, None);
                    if !run {
                        *control_flow = ControlFlow::Exit;
                    }

                    let gl_window = display.gl_window();
                    let mut target = display.draw();
                    target.clear_color_srgb(0.3, 0.5, 0.5, 1.0);
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

                        let mut run = true;
                        match &event {
                            Event::WindowEvent { event, .. } => run_ui(
                                &mut run,
                                &mut ui,
                                &display,
                                &mut renderer,
                                Some(event),
                                None,
                            ),
                            Event::UserEvent(event) => run_ui(
                                &mut run,
                                &mut ui,
                                &display,
                                &mut renderer,
                                None,
                                Some(event),
                            ),
                            _ => run_ui(&mut run, &mut ui, &display, &mut renderer, None, None),
                        };

                        if !run {
                            *control_flow = ControlFlow::Exit;
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
