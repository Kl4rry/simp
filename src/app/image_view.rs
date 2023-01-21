use std::{
    borrow::Cow,
    mem,
    path::PathBuf,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use cgmath::{Deg, Matrix4, Ortho, Vector3, Vector4};
use glium::{
    backend::glutin::Display,
    draw_parameters::DrawParameters,
    glutin::event_loop::EventLoopProxy,
    implement_vertex,
    index::PrimitiveType,
    program::Program,
    texture::{ClientFormat, MipmapsOption, RawImage2d, SrgbTexture2d},
    uniform,
    uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler, SamplerBehavior},
    Blend, IndexBuffer, Surface, VertexBuffer,
};
use image::{DynamicImage, GenericImageView};
use once_cell::sync::OnceCell;

use super::op_queue::{Output, UserEventLoopProxyExt};
use crate::{
    max,
    rect::Rect,
    util::{ColorBits, GetColorBits, Image, ImageData, UserEvent},
    vec2::Vec2,
};

mod crop;
use crop::Crop;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}

impl Vertex {
    pub fn new(x: f32, y: f32, tex_x: f32, tex_y: f32) -> Self {
        Self {
            position: [x, y],
            tex_coords: [tex_x, tex_y],
        }
    }
}

implement_vertex!(Vertex, position, tex_coords);

pub struct ImageView {
    pub size: Vec2<f32>,
    pub position: Vec2<f32>,
    pub scale: f32,
    rotation: i32,
    pub path: Option<PathBuf>,
    pub image_data: Arc<RwLock<Arc<ImageData>>>,
    pub last_frame: Instant,
    pub index: usize,
    pub horizontal_flip: bool,
    pub vertical_flip: bool,
    pub hue: f32,
    pub contrast: f32,
    pub brightness: f32,
    pub saturation: f32,
    pub grayscale: bool,
    pub invert: bool,
    pub crop: Crop,
    vertices: VertexBuffer<Vertex>,
    indices: IndexBuffer<u8>,
    texture: SrgbTexture2d,
    sampler: SamplerBehavior,
}

impl ImageView {
    pub fn new(display: &Display, image_data: Arc<ImageData>, path: Option<PathBuf>) -> Self {
        let frames = &image_data.frames;
        let image = frames[0].buffer();
        let (width, height) = image.dimensions();
        let texture = get_texture(image, display);

        let sampler = SamplerBehavior {
            magnify_filter: MagnifySamplerFilter::Nearest,
            minify_filter: MinifySamplerFilter::LinearMipmapLinear,
            max_anisotropy: 8,
            ..Default::default()
        };

        Self {
            size: Vec2::new(width as f32, height as f32),
            position: Vec2::default(),
            scale: 1.0,
            rotation: 0,
            image_data: Arc::new(RwLock::new(image_data)),
            last_frame: Instant::now(),
            index: 0,
            horizontal_flip: false,
            vertical_flip: false,
            crop: Crop::new(display),
            vertices: get_vertex_buffer(display, width as f32, height as f32),
            indices: IndexBuffer::new(display, PrimitiveType::TrianglesList, &[0, 1, 2, 2, 1, 3])
                .unwrap(),
            texture,
            sampler,
            path,
            hue: 0.0,
            contrast: 0.0,
            brightness: 0.0,
            saturation: 0.0,
            grayscale: false,
            invert: false,
        }
    }

    pub fn get_rotation_mat(&self) -> Matrix4<f32> {
        let rotation = Matrix4::from_angle_z(Deg((self.rotation * 90) as f32));
        let pre_rotation =
            Matrix4::from_translation(Vector3::new(self.size.x() / 2.0, self.size.y() / 2.0, 0.0));
        let post_rotation = Matrix4::from_translation(Vector3::new(
            -self.size.x() / 2.0,
            -self.size.y() / 2.0,
            0.0,
        ));
        (pre_rotation * rotation) * post_rotation
    }

    pub fn render(&self, target: &mut glium::Frame, display: &Display, size: Vec2<f32>) {
        let ortho: Matrix4<f32> = Ortho {
            left: 0.0,
            right: size.x(),
            bottom: size.y(),
            top: 0.0,
            near: 0.0,
            far: 1.0,
        }
        .into();

        let position = self.position - self.scaled() / 2.0;
        let scale = Matrix4::from_scale(self.scale);
        let translation = Matrix4::from_translation(Vector3::new(position.x(), position.y(), 0.0));

        let rotation = self.get_rotation_mat();
        let matrix = ortho * translation * scale * rotation;

        let raw: [[f32; 4]; 4] = matrix.into();

        thread_local! {
            pub static PROGRAM: OnceCell<Program> = OnceCell::new();
        }

        PROGRAM.with(|f| {
            let shader = f.get_or_init(|| {
                Program::from_source(
                    display,
                    include_str!("../shader/image.vert"),
                    include_str!("../shader/image.frag"),
                    None,
                )
                .unwrap()
            });

            target
                .draw(
                    &self.vertices,
                    &self.indices,
                    shader,
                    &uniform! {
                        matrix: raw,
                        tex: Sampler(&self.texture, self.sampler),
                        size: size,
                        hue: self.hue,
                        contrast: self.contrast,
                        brightness: self.brightness,
                        saturation: self.saturation,
                        grayscale: self.grayscale,
                        invert: self.invert,
                        flip_horizontal: self.horizontal_flip,
                        flip_vertical: self.vertical_flip,
                    },
                    &DrawParameters {
                        blend: Blend::alpha_blending(),
                        ..DrawParameters::default()
                    },
                )
                .unwrap();
        });

        self.crop.render(
            target,
            display,
            size,
            self.position,
            self.rotated_size(),
            self.scale,
        );
    }

    pub fn scaled(&self) -> Vec2<f32> {
        self.size * self.scale
    }

    pub fn real_size(&self) -> Vec2<f32> {
        let mut vectors = vec![
            Vector4::new(0.0, 0.0, 0.0, 1.0),
            Vector4::new(0.0, self.size.y(), 0.0, 1.0),
            Vector4::new(self.size.x(), 0.0, 0.0, 1.0),
            Vector4::new(self.size.x(), self.size.y(), 0.0, 1.0),
        ];

        let rotation = Matrix4::from_angle_z(Deg((self.rotation * 90) as f32));

        let scale = Matrix4::from_scale(self.scale);

        let matrix = scale * rotation;

        for vector in &mut vectors {
            (*vector) = matrix * (*vector);
        }

        let mut size = Vec2::new(0.0, 0.0);

        for outer in &vectors {
            for inner in &vectors {
                size.set_x(max!((inner.x - outer.x).abs(), size.x()));
                size.set_y(max!((inner.y - outer.y).abs(), size.y()));
            }
        }

        size
    }

    pub fn flip_horizontal(&mut self) {
        self.horizontal_flip = !self.horizontal_flip;
    }

    pub fn flip_vertical(&mut self) {
        self.vertical_flip = !self.vertical_flip;
    }

    pub fn animate(&mut self, display: &Display) -> Option<Duration> {
        let guard = self.image_data.read().unwrap();
        let frames = &guard.frames;
        if frames.len() > 1 {
            let now = Instant::now();
            let time_passed = now.duration_since(self.last_frame);
            let delay = frames[self.index].delay;

            if time_passed > delay {
                self.index += 1;
                if self.index >= frames.len() {
                    self.index = 0;
                }

                drop(guard);
                self.update_image_data(display);

                self.last_frame = now;

                Some(delay)
            } else {
                Some(delay - time_passed)
            }
        } else {
            None
        }
    }

    pub fn crop(&self, cut: Rect, proxy: EventLoopProxy<UserEvent>) {
        let rotation = self.rotation;
        let image_data = self.image_data.clone();
        let old_rotation = self.rotation;
        thread::spawn(move || {
            let mut new_frames = Vec::new();
            let guard = image_data.read().unwrap();
            let frames = &guard.frames;
            for frame in frames {
                let buffer = match rotation {
                    0 => frame.buffer().clone(),
                    1 => frame.buffer().rotate90(),
                    2 => frame.buffer().rotate180(),
                    3 => frame.buffer().rotate270(),
                    _ => unreachable!(),
                };

                let image = buffer.crop_imm(
                    cut.x() as u32,
                    cut.y() as u32,
                    cut.width() as u32,
                    cut.height() as u32,
                );

                new_frames.push(Image::with_delay(image, frame.delay));
            }

            proxy.send_output(Output::Crop(new_frames, old_rotation));
        });
    }

    pub fn swap_frames(&mut self, frames: &mut Vec<Image>, display: &Display) {
        let mut guard = self.image_data.write().unwrap();
        mem::swap(&mut Arc::make_mut(&mut *guard).frames, frames);
        let (width, height) = guard.frames[0].buffer().dimensions();
        drop(guard);
        self.update_image_data(display);
        self.vertices = get_vertex_buffer(display, width as f32, height as f32);
    }

    fn update_image_data(&mut self, display: &Display) {
        let guard = self.image_data.read().unwrap();
        let frames = &guard.frames;
        let image = frames[self.index].buffer();
        self.size = Vec2::new(image.width() as f32, image.height() as f32);
        self.texture = get_texture(image, display);
    }

    pub fn rotation(&self) -> i32 {
        self.rotation
    }

    pub fn rotate(&mut self, rot: i32) {
        self.rotation += rot;
        self.rotation %= 4;
        while self.rotation < 0 {
            self.rotation += 4;
        }
    }

    pub fn set_rotation(&mut self, rot: i32) {
        self.rotation = rot % 4;
    }

    pub fn swap_rotation(&mut self, rot: &mut i32) {
        std::mem::swap(&mut self.rotation, rot);
        self.rotation %= 4;
    }

    pub fn rotated_size(&self) -> Vec2<f32> {
        let mut size = self.size;
        if self.rotation % 2 != 0 {
            size.swap();
        }
        size
    }

    pub fn cropping(&self) -> bool {
        self.crop.cropping()
    }

    pub fn start_crop(&mut self) {
        let size = self.rotated_size();
        let position = Vec2::new(0.0, 0.0);
        self.crop.rect = Some(Rect { position, size });
        self.crop.x = position.x().to_string();
        self.crop.y = position.y().to_string();
        self.crop.width = size.x().to_string();
        self.crop.height = size.y().to_string();
    }

    pub fn cancel_crop(&mut self) {
        self.crop.rect = None;
    }

    pub fn handle_drag(&mut self, ui: &egui::Ui) {
        let dragging = self
            .crop
            .handle_drag(ui, self.position, self.rotated_size(), self.scale);

        let res = ui.interact(egui::Rect::EVERYTHING, ui.id(), egui::Sense::drag());
        if (res.dragged_by(egui::PointerButton::Primary) && !dragging)
            || res.dragged_by(egui::PointerButton::Middle)
        {
            let vec2 = res.drag_delta();
            self.position += Vec2::from((vec2.x, vec2.y));
        }
    }
}

fn get_texture(image: &DynamicImage, display: &Display) -> SrgbTexture2d {
    let (width, height) = image.dimensions();

    match image {
        DynamicImage::ImageRgb8(buffer) => {
            let data = Cow::Borrowed(&buffer.as_raw()[..]);
            let raw = RawImage2d {
                data,
                width,
                height,
                format: ClientFormat::U8U8U8,
            };
            SrgbTexture2d::with_mipmaps(display, raw, MipmapsOption::AutoGeneratedMipmaps).unwrap()
        }
        DynamicImage::ImageRgba8(buffer) => {
            let data = Cow::Borrowed(&buffer.as_raw()[..]);
            let raw = RawImage2d {
                data,
                width,
                height,
                format: ClientFormat::U8U8U8U8,
            };
            SrgbTexture2d::with_mipmaps(display, raw, MipmapsOption::AutoGeneratedMipmaps).unwrap()
        }
        DynamicImage::ImageRgb16(buffer) => {
            let data = Cow::Borrowed(&buffer.as_raw()[..]);
            let raw = RawImage2d {
                data,
                width,
                height,
                format: ClientFormat::U16U16U16,
            };
            SrgbTexture2d::with_mipmaps(display, raw, MipmapsOption::AutoGeneratedMipmaps).unwrap()
        }
        DynamicImage::ImageRgba16(buffer) => {
            let data = Cow::Borrowed(&buffer.as_raw()[..]);
            let raw = RawImage2d {
                data,
                width,
                height,
                format: ClientFormat::U16U16U16U16,
            };
            SrgbTexture2d::with_mipmaps(display, raw, MipmapsOption::AutoGeneratedMipmaps).unwrap()
        }
        DynamicImage::ImageRgb32F(buffer) => {
            let data = Cow::Borrowed(&buffer.as_raw()[..]);
            let raw = RawImage2d {
                data,
                width,
                height,
                format: ClientFormat::F32F32F32,
            };
            SrgbTexture2d::with_mipmaps(display, raw, MipmapsOption::AutoGeneratedMipmaps).unwrap()
        }
        DynamicImage::ImageRgba32F(buffer) => {
            let data = Cow::Borrowed(&buffer.as_raw()[..]);
            let raw = RawImage2d {
                data,
                width,
                height,
                format: ClientFormat::F32F32F32F32,
            };
            SrgbTexture2d::with_mipmaps(display, raw, MipmapsOption::AutoGeneratedMipmaps).unwrap()
        }
        image => match image.get_color_bits() {
            ColorBits::U8 => {
                let data = Cow::Owned(image.to_rgba8().into_raw());
                let raw = RawImage2d {
                    data,
                    width,
                    height,
                    format: ClientFormat::U8U8U8U8,
                };
                SrgbTexture2d::with_mipmaps(display, raw, MipmapsOption::AutoGeneratedMipmaps)
                    .unwrap()
            }
            ColorBits::U16 => {
                let data = Cow::Owned(image.to_rgba16().into_raw());
                let raw = RawImage2d {
                    data,
                    width,
                    height,
                    format: ClientFormat::U16U16U16U16,
                };
                SrgbTexture2d::with_mipmaps(display, raw, MipmapsOption::AutoGeneratedMipmaps)
                    .unwrap()
            }
            ColorBits::F32 => {
                let data = Cow::Owned(image.to_rgba32f().into_raw());
                let raw = RawImage2d {
                    data,
                    width,
                    height,
                    format: ClientFormat::U16U16U16U16,
                };
                SrgbTexture2d::with_mipmaps(display, raw, MipmapsOption::AutoGeneratedMipmaps)
                    .unwrap()
            }
        },
    }
}

fn get_vertex_buffer(display: &Display, width: f32, height: f32) -> VertexBuffer<Vertex> {
    let texture_cords = (
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0),
    );
    let shape = vec![
        Vertex::new(0.0, 0.0, texture_cords.0.x(), texture_cords.0.y()),
        Vertex::new(0.0, height, texture_cords.1.x(), texture_cords.1.y()),
        Vertex::new(width, 0.0, texture_cords.2.x(), texture_cords.2.y()),
        Vertex::new(width, height, texture_cords.3.x(), texture_cords.3.y()),
    ];
    VertexBuffer::new(display, &shape).unwrap()
}
