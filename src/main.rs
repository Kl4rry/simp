#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(clippy::all)]

use std::{
    env,
    io::{self, IsTerminal, Read},
    iter, panic,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use cgmath::Vector2;
use cli::get_clap_command;
use egui::ViewportId;
use serde::{Deserialize, Serialize};

mod app;
use app::{
    op_queue::Op,
    preferences::{Preferences, PREFERENCES},
    App,
};
mod icon;
mod rect;
mod util;
use util::UserEvent;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key, ModifiersState},
    window::{Fullscreen, Window, WindowAttributes},
};
mod cli;
mod image_io;

#[derive(Serialize, Deserialize, Debug, Default)]
struct Config {
    maximized: bool,
    preferences: Preferences,
}

pub struct WgpuState {
    pub window: Arc<Window>,
    pub adapter: wgpu::Adapter,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub scale_factor: f64,
}

pub struct WindowHandler {
    pub wgpu: WgpuState,
    pub event_loop: EventLoop<UserEvent>,
    pub proxy: EventLoopProxy<UserEvent>,
    pub egui_winit: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
    pub egui_shapes: Vec<egui::ClippedPrimitive>,
    pub app: App,
}

impl WindowHandler {
    pub async fn new(class: &str, fullscreen: bool, zen_mode: bool, no_cache: bool) -> Self {
        let mut config: Config = confy::load("simp", None).unwrap_or_default();
        config.preferences.clamp();

        let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event().build().unwrap();
        let proxy = event_loop.create_proxy();

        let fullscreen = if fullscreen || config.preferences.open_in_fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        };

        *PREFERENCES.lock().unwrap() = config.preferences;

        let builder = WindowAttributes::default()
            .with_title(String::from("Simp"))
            .with_visible(false)
            .with_min_inner_size(winit::dpi::LogicalSize::new(640f64, 400f64))
            .with_inner_size(winit::dpi::LogicalSize::new(1100f64, 720f64))
            .with_maximized(config.maximized)
            .with_fullscreen(fullscreen)
            .with_window_icon(Some(icon::get_icon()));

        #[cfg(all(unix, not(target_os = "macos")))]
        let builder = {
            use winit::platform::{wayland, x11};
            let builder = wayland::WindowAttributesExtWayland::with_name(builder, class, class);
            x11::WindowAttributesExtX11::with_name(builder, class, class)
        };
        let _ = class;

        #[allow(deprecated)]
        let window = Arc::new(event_loop.create_window(builder).unwrap());

        let size = window.inner_size();

        let mut backends = if cfg!(windows) {
            wgpu::Backends::DX12
        } else if cfg!(target_os = "macos") {
            wgpu::Backends::PRIMARY
        } else {
            wgpu::Backends::all()
        };

        if let Ok(gpu_backend) = env::var("SIMP_GPU_BACKEND") {
            backends = wgpu::util::parse_backends_from_comma_list(&gpu_backend);
        } else if let Ok(gpu_backend) = env::var("WGPU_BACKEND") {
            backends = wgpu::util::parse_backends_from_comma_list(&gpu_backend);
        };

        let instance_descriptor = wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        };

        let instance = wgpu::Instance::new(instance_descriptor);
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Unable to create adapter");

        let limits = wgpu::Limits::default();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::default(),
                    required_limits: limits.clone(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let mut egui_winit = egui_winit::State::new(
            egui::Context::default(),
            ViewportId::ROOT,
            &window.clone(),
            None,
            None,
            None,
        );
        egui_winit.set_max_texture_side(limits.max_texture_dimension_2d as usize);
        let egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1, false);
        let egui_shapes = Vec::new();

        {
            let repaint_proxy = Arc::new(Mutex::new(event_loop.create_proxy()));
            egui_winit
                .egui_ctx()
                .set_request_repaint_callback(move |info| {
                    let _ = repaint_proxy
                        .lock()
                        .unwrap()
                        .send_event(UserEvent::RepaintRequest(info));
                });
        }

        egui_winit.egui_ctx().style_mut(|style| {
            style.spacing.slider_width = 200.0;
        });

        let wgpu = WgpuState {
            window: window.clone(),
            adapter,
            surface,
            device,
            queue,
            config,
            scale_factor: 1.0,
        };

        let app = App::new(
            &wgpu,
            proxy.clone(),
            [size.width as f32, size.height as f32],
            zen_mode,
            no_cache,
        );

        let ctrl_proxy = proxy.clone();
        ctrlc::set_handler(move || {
            let _ = ctrl_proxy.send_event(UserEvent::Exit);
        })
        .unwrap();

        Self {
            event_loop,
            proxy,
            app,
            wgpu,
            egui_winit,
            egui_renderer,
            egui_shapes,
        }
    }
}

impl WindowHandler {
    pub fn main_loop(mut self) {
        let WindowHandler {
            event_loop,
            mut app,
            proxy,
            mut egui_winit,
            mut egui_renderer,
            ref mut egui_shapes,
            mut wgpu,
        } = self;

        #[allow(deprecated)]
        let _ = event_loop.run(move |event, event_loop| match event {
            Event::Resumed => wgpu.window.set_visible(true),
            Event::NewEvents(..) => wgpu.window.request_redraw(),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    let _ = proxy.send_event(UserEvent::Exit);
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    wgpu.scale_factor = scale_factor;
                }
                WindowEvent::Resized(size) => {
                    if size.width > 0 && size.height > 0 {
                        wgpu.config.width = size.width;
                        wgpu.config.height = size.height;
                        wgpu.surface.configure(&wgpu.device, &wgpu.config);
                        app.resize(size);
                    }
                }
                WindowEvent::RedrawRequested => {
                    {
                        let raw_input = egui_winit.take_egui_input(&wgpu.window);
                        let egui_output = egui_winit.egui_ctx().run(raw_input, |ctx| {
                            app.handle_ui(&wgpu, ctx);
                        });
                        egui_winit
                            .handle_platform_output(&wgpu.window, egui_output.platform_output);
                        for (id, image_delta) in egui_output.textures_delta.set {
                            egui_renderer.update_texture(
                                &wgpu.device,
                                &wgpu.queue,
                                id,
                                &image_delta,
                            );
                        }

                        for id in egui_output.textures_delta.free {
                            egui_renderer.free_texture(&id);
                        }

                        let pixels_per_point = egui_winit.egui_ctx().pixels_per_point();
                        *egui_shapes = egui_winit
                            .egui_ctx()
                            .tessellate(egui_output.shapes, pixels_per_point);
                    }

                    let (exit, repaint_after) = app.update(&wgpu);

                    if exit {
                        event_loop.exit();
                    }

                    let control_flow = ControlFlow::wait_duration(repaint_after);
                    event_loop.set_control_flow(control_flow);

                    let output = wgpu.surface.get_current_texture().unwrap();
                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder =
                        wgpu.device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("Render Encoder"),
                            });

                    {
                        {
                            let clear_color = ((44_f64) / 255.0).powf(2.2);
                            let mut rpass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("Image render pass"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                                r: clear_color,
                                                g: clear_color,
                                                b: clear_color,
                                                a: 1.0,
                                            }),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                });

                            if let Some(image) = app.image_view.as_mut() {
                                let uniform = image.get_uniform(Vector2::new(
                                    wgpu.config.width as f32,
                                    wgpu.config.height as f32,
                                ));
                                app.image_renderer.prepare(&wgpu, uniform);
                                app.image_renderer.render(&mut rpass, image);

                                let window_size = Vector2::new(
                                    wgpu.config.width as f32,
                                    wgpu.config.height as f32,
                                );
                                if let Some(uniform) = image.crop.get_uniform(
                                    window_size,
                                    image.position,
                                    image.rotated_size(),
                                    image.scale,
                                ) {
                                    app.crop_renderer.prepare(&wgpu, uniform);
                                    app.crop_renderer.render(&mut rpass);
                                }
                            }
                        }

                        {
                            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                                pixels_per_point: wgpu.window.scale_factor() as f32,
                                size_in_pixels: [wgpu.config.width, wgpu.config.height],
                            };

                            let cmd_buffers = egui_renderer.update_buffers(
                                &wgpu.device,
                                &wgpu.queue,
                                &mut encoder,
                                egui_shapes,
                                &screen_descriptor,
                            );
                            wgpu.queue.submit(cmd_buffers);

                            let mut pass = encoder
                                .begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("Gui Render Pass"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Load,
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    occlusion_query_set: None,
                                    timestamp_writes: None,
                                })
                                .forget_lifetime();

                            egui_renderer.render(&mut pass, egui_shapes, &screen_descriptor);
                        }

                        wgpu.queue.submit(iter::once(encoder.finish()));
                        wgpu.window.pre_present_notify();
                        output.present();
                    }
                }
                event => {
                    match &event {
                        WindowEvent::ModifiersChanged(modifiers) => {
                            app.modifiers = modifiers.state()
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            if app.modifiers.contains(ModifiersState::CONTROL) {
                                if let Key::Character(ch) = &event.logical_key {
                                    match ch.as_str() {
                                        "v" => app.handle_paste(),
                                        // Remove the scufffed egui zoom
                                        "+" => return,
                                        "-" => return,
                                        _ => (),
                                    }
                                }
                            }
                        }
                        _ => (),
                    }

                    let res = egui_winit.on_window_event(&wgpu.window, &event);
                    if !res.consumed || matches!(event, WindowEvent::ModifiersChanged(_)) {
                        app.handle_window_event(&wgpu, &event);
                    }

                    if res.repaint {
                        wgpu.window.request_redraw();
                    }
                }
            },
            Event::LoopExiting => {
                wgpu.window.set_visible(false);
                let data = Config {
                    maximized: wgpu.window.is_maximized(),
                    preferences: PREFERENCES.lock().unwrap().clone(),
                };
                confy::store("simp", None, data).unwrap();
                std::process::exit(0);
            }
            Event::UserEvent(mut event) => app.handle_user_event(&wgpu, &mut event),
            _ => (),
        });
    }
}

fn generate_man() {
    let cmd = get_clap_command();
    let man = clap_mangen::Man::new(cmd);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer).unwrap();
    std::io::Write::write_all(&mut std::io::stdout(), &buffer).unwrap();
}

fn main() {
    panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let panic_info = format!("{backtrace}\n{info}");
        let _ = std::fs::write("panic.txt", &panic_info);
        println!("{}", panic_info);
    }));

    let matches = cli::get_clap_command().get_matches();

    if matches.get_flag("generate-man") {
        generate_man();
        return;
    }

    let path: Option<&String> = matches.get_one("file");
    let fullscreen: bool = matches.get_flag("fullscreen");
    let zen_mode: bool = matches.get_flag("zen-mode");
    let no_cache: bool = matches.get_flag("no-cache");
    let class: &String = matches.get_one("class").unwrap();
    let mut window_handler =
        pollster::block_on(WindowHandler::new(class, fullscreen, zen_mode, no_cache));

    if !io::stdin().is_terminal() {
        let proxy = window_handler.proxy.clone();
        thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = io::stdin().read_to_end(&mut buffer);
            if !buffer.is_empty() {
                let _ = proxy.send_event(UserEvent::LoadBytes(buffer));
            }
        });
    }

    if let Some(path) = path {
        window_handler
            .app
            .queue(Op::LoadPath(PathBuf::from(path), true))
    }
    window_handler.main_loop();
}
