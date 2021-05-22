use glium::{
    backend::Facade,
    glutin::{
        event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
        event_loop::EventLoopProxy,
    },
    texture::{ClientFormat, RawImage2d},
    uniforms::{MagnifySamplerFilter, MinifySamplerFilter, SamplerBehavior},
    Texture2d,
};
use image::{io::Reader as ImageReader, ImageBuffer, Rgba};
use imgui::*;
use imgui_glium_renderer::{Renderer, Texture};
use std::{borrow::Cow, env, error::Error, fs, io::Cursor, path::Path, rc::Rc, thread};

mod window;
use window::{System, UserEvent};
mod vec2;
use vec2::Vec2;
mod background;
use background::render_background;

macro_rules! min {
    ($x: expr) => ($x);
    ($x: expr, $($z: expr),+) => {{
        let y = min!($($z),*);
        if $x < y {
            $x
        } else {
            y
        }
    }}
}

struct ImageView {
    texture_id: TextureId,
    size: Vec2<f32>,
    position: Vec2<f32>,
    scale: f32,
}

impl ImageView {
    fn new<F>(
        gl_ctx: &F,
        textures: &mut Textures<Texture>,
        image_buffer: ImageBuffer<Rgba<u16>, Vec<u16>>,
    ) -> Result<Self, Box<dyn Error>>
    where
        F: Facade,
    {
        let (width, height) = image_buffer.dimensions();
        let raw = RawImage2d {
            data: Cow::Owned(image_buffer.into_raw()),
            width: width as u32,
            height: height as u32,
            format: ClientFormat::U16U16U16U16,
        };

        let gl_texture = Texture2d::new(gl_ctx, raw)?;
        let texture = Texture {
            texture: Rc::new(gl_texture),
            sampler: SamplerBehavior {
                magnify_filter: MagnifySamplerFilter::Nearest,
                minify_filter: MinifySamplerFilter::Linear,
                ..Default::default()
            },
        };
        let texture_id = textures.insert(texture);
        Ok(ImageView {
            texture_id,
            size: Vec2::new(width as f32, height as f32),
            scale: 1.0,
            position: Vec2::default(),
        })
    }

    fn scaled(&self) -> Vec2<f32> {
        self.size * self.scale
    }
}

struct App {
    image_view: Option<ImageView>,
    mouse_down: bool,
    last_mouse_position: Vec2<f32>,
    size: Vec2<f32>,
    proxy: EventLoopProxy<UserEvent>,
}

impl App {
    fn update(
        &mut self,
        ui: &Ui,
        display: &glium::Display,
        renderer: &mut Renderer,
        window_event: Option<&WindowEvent>,
        user_event: Option<&UserEvent>,
    ) {
        let styles = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));

        if let Some(event) = user_event {
            match event {
                UserEvent::ImageLoaded(image) => {
                    self.image_view = Some(
                        ImageView::new(display.get_context(), renderer.textures(), image.clone())
                            .unwrap(),
                    );
                    let view = self.image_view.as_mut().unwrap();

                    let scaling = min!(self.size.x() / view.size.x(), (self.size.y() - 50.0) / view.size.y());
                    view.scale = scaling;
                    view.position = self.size / 2.0;
                }
            };
        }

        if let Some(event) = window_event {
            match event {
                WindowEvent::Resized(size) => {
                    self.size = Vec2::new(size.width as f32, size.height as f32)
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if let Some(ref mut image) = self.image_view {
                        let old_scale = match delta {
                            MouseScrollDelta::LineDelta(_, y) => {
                                let old_scale = image.scale;
                                image.scale += image.scale * y / 10.0;
                                old_scale
                            }
                            MouseScrollDelta::PixelDelta(pos) => {
                                let old_scale = image.scale;
                                image.scale += image.scale * pos.y as f32 / 10.0;
                                old_scale
                            }
                        };
                        let new_size = image.scaled();
                        if new_size.x() < 100.0 || new_size.y() < 100.0 {
                            image.scale = old_scale;
                        }
                    }
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    if matches!(button, MouseButton::Left) {
                        match state {
                            ElementState::Released => self.mouse_down = false,
                            ElementState::Pressed => self.mouse_down = true,
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(ref mut image) = self.image_view {
                        let pos = Vec2::new(position.x as f32, position.y as f32);
                        if self.mouse_down {
                            let differance = self.last_mouse_position - pos;
                            image.position -= differance;
                        }
                        self.last_mouse_position = pos;
                    }
                }
                _ => (),
            };
        }

        if let Some(ref mut image) = self.image_view {
            let image_size = image.scaled();
            let mut window_size = self.size;
            window_size.set_y(window_size.y() - 50.0);

            if image_size.x() < window_size.x() {
                if image.position.x() - image_size.x() / 2.0 < 0.0 {
                    image.position.set_x(image_size.x() / 2.0);
                }

                if image.position.x() + image_size.x() / 2.0 > window_size.x() {
                    image.position.set_x(window_size.x() - image_size.x() / 2.0);
                }
            } else {
                if image.position.x() - image_size.x() / 2.0 > 0.0 {
                    image.position.set_x(image_size.x() / 2.0);
                }

                if image.position.x() + image_size.x() / 2.0 < window_size.x() {
                    image.position.set_x(window_size.x() - image_size.x() / 2.0);
                }
            }

            if image_size.y() < window_size.y() {
                if image.position.y() - image_size.y() / 2.0 < 50.0 {
                    image.position.set_y((image_size.y() / 2.0) + 50.0);
                }

                if image.position.y() + image_size.y() / 2.0 - 50.0 > window_size.y() {
                    image.position.set_y((window_size.y() - image_size.y() / 2.0) + 50.0);
                }
            } else {
                if image.position.y() - image_size.y() / 2.0 > 50.0 {
                    image.position.set_y(image_size.y() / 2.0 + 50.0);
                }

                if image.position.y() + image_size.y() / 2.0 < window_size.y() + 50.0 {
                    image.position.set_y((window_size.y() - image_size.y() / 2.0) + 50.0);
                }
            }
        }

        Window::new(im_str!("window"))
            .size(*self.size, Condition::Always)
            .position([0.0, 0.0], Condition::Always)
            .bg_alpha(0.0)
            .no_decoration()
            .draw_background(false)
            .scrollable(false)
            .movable(false)
            .build(ui, || {

                render_background(ui, &self.size);

                Window::new(im_str!("controls"))
                    .size([self.size.x(), 50.0], Condition::Always)
                    .position([0.0, 0.0], Condition::Always)
                    .bg_alpha(1.0)
                    .no_decoration()
                    .scrollable(false)
                    .movable(false)
                    .bring_to_front_on_focus(true)
                    .focused(true)
                    .no_nav()
                    .build(ui, || {
                        if ui.button(im_str!("Browse"), [70.0, 50.0]) {
                            let proxy = self.proxy.clone();
                            thread::spawn(move || {
                                if let Some(file) = tinyfiledialogs::open_file_dialog("Open image", "", None) {
                                    load_image(proxy, file);
                                } 
                            });
                        }
                    });

                

                if let Some(ref mut image) = self.image_view {
                    Window::new(im_str!("image"))
                        .size(*image.scaled(), Condition::Always)
                        .position_pivot([0.5, 0.5])
                        .position(*image.position, Condition::Always)
                        .bg_alpha(0.0)
                        .no_decoration()
                        .scrollable(false)
                        .draw_background(false)
                        .mouse_inputs(false)
                        .focus_on_appearing(false)
                        .no_nav()
                        .build(ui, || {
                            Image::new(image.texture_id, *image.scaled()).build(ui)
                        });
                }
            });

        styles.pop(&ui);
    }

    fn new(system: &mut System, size: [f32; 2]) -> Self {
        App {
            image_view: None,
            mouse_down: false,
            last_mouse_position: Vec2::default(),
            size: Vec2::new(size[0], size[1]),
            proxy: system.proxy.clone(),
        }
    }
}

fn main() {
    let mut system = window::init();
    let dimensions = system.display.get_framebuffer_dimensions();
    let size = [dimensions.0 as f32, dimensions.1 as f32];
    let mut app = App::new(&mut system, size);

    let mut args: Vec<String> = env::args().collect();
    if let Some(arg) = args.pop() {
        load_image(system.proxy.clone(), arg);
    }

    system.main_loop(move |_, ui, display, renderer, window_event, user_event| {
        app.update(ui, display, renderer, window_event, user_event)
    });
}

fn decode_image(
    path: impl AsRef<Path>,
) -> Result<ImageBuffer<Rgba<u16>, Vec<u16>>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(
        ImageReader::with_format(Cursor::new(&bytes), image::guess_format(&bytes)?)
            .decode()?
            .into_rgba16(),
    )
}

fn load_image(proxy: EventLoopProxy<UserEvent>, path: impl AsRef<Path>) {
    let path_buf = path.as_ref().to_path_buf();
    thread::spawn(move || {
        if let Ok(image) = decode_image(path_buf) {
            let _ = proxy.send_event(UserEvent::ImageLoaded(image));
        }
    });
}
