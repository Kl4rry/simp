use std::{
    borrow::Cow,
    mem,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use cgmath::{Matrix4, Ortho, Vector3, Vector4};
use glium::{
    backend::glutin::Display,
    draw_parameters::DrawParameters,
    implement_vertex,
    index::PrimitiveType,
    program::Program,
    texture::{ClientFormat, MipmapsOption, RawImage2d, SrgbTexture2d},
    uniform,
    uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler, SamplerBehavior},
    Blend, IndexBuffer, Surface, VertexBuffer,
};
use image::{imageops::rotate180_in_place, DynamicImage, GenericImageView};
use rect::Rect;
use util::{max, min, Image};
use vec2::Vec2;

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
    pub rotation: i32,
    pub path: Option<PathBuf>,
    pub start: Instant,
    pub frames: Arc<RwLock<Vec<Image>>>,
    pub last_frame: Instant,
    pub index: usize,
    pub horizontal_flip: bool,
    pub vertical_flip: bool,
    shader: Box<Program>,
    vertices: VertexBuffer<Vertex>,
    indices: IndexBuffer<u8>,
    texture: SrgbTexture2d,
    texture_cords: (Vec2<f32>, Vec2<f32>, Vec2<f32>, Vec2<f32>),
    sampler: SamplerBehavior,
}

impl ImageView {
    pub fn new(
        display: &Display,
        frames: Arc<RwLock<Vec<Image>>>,
        path: Option<PathBuf>,
        start: Instant,
    ) -> Self {
        let guard = frames.read().unwrap();
        let image = guard[0].buffer();
        let (width, height) = image.dimensions();
        let texture_cords = (
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
        );
        let shape = vec![
            Vertex::new(0.0, 0.0, texture_cords.0.x(), texture_cords.0.y()),
            Vertex::new(0.0, height as f32, texture_cords.1.x(), texture_cords.1.y()),
            Vertex::new(width as f32, 0.0, texture_cords.2.x(), texture_cords.2.y()),
            Vertex::new(
                width as f32,
                height as f32,
                texture_cords.3.x(),
                texture_cords.3.y(),
            ),
        ];
        let index_buffer = &[0, 1, 2, 2, 1, 3];

        let texture = get_texture(image, display);

        drop(guard);

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
            frames,
            last_frame: Instant::now(),
            index: 0,
            start,
            horizontal_flip: false,
            vertical_flip: false,
            shader: Box::new(
                Program::from_source(
                    display,
                    include_str!("../shader/image.vert"),
                    include_str!("../shader/image.frag"),
                    None,
                )
                .unwrap(),
            ),
            vertices: VertexBuffer::new(display, &shape).unwrap(),
            indices: IndexBuffer::new(display, PrimitiveType::TrianglesList, index_buffer).unwrap(),
            texture,
            texture_cords,
            sampler,
            path,
        }
    }

    pub fn render(&self, target: &mut glium::Frame, size: Vec2<f32>) {
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
        let translation =
            Matrix4::from_translation(cgmath::Vector3::new(position.x(), position.y(), 0.0));

        let rotation = get_rotation_matrix(degrees_to_radians((self.rotation * 90) as f32));

        let pre_rotation =
            Matrix4::from_translation(Vector3::new(self.size.x() / 2.0, self.size.y() / 2.0, 0.0));
        let post_rotation = Matrix4::from_translation(Vector3::new(
            -self.size.x() / 2.0,
            -self.size.y() / 2.0,
            0.0,
        ));
        let final_rotation = (pre_rotation * rotation) * post_rotation;

        let matrix = ortho * translation * scale * final_rotation;

        let raw: [[f32; 4]; 4] = matrix.into();

        target
            .draw(
                &self.vertices,
                &self.indices,
                &self.shader,
                &uniform! { matrix: raw, tex: Sampler(&self.texture, self.sampler) },
                &DrawParameters {
                    blend: Blend::alpha_blending(),
                    ..DrawParameters::default()
                },
            )
            .unwrap();
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

        let rot = degrees_to_radians((self.rotation * 90) as f32);

        #[rustfmt::skip]
        let rotation = Matrix4::new(
            rot.cos(), -(rot.sin()), 0.0, 0.0,
            rot.sin(), rot.cos(), 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        );

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

    pub fn bounds(&self) -> Rect {
        let mut vectors = [
            Vector4::new(0.0, 0.0, 0.0, 1.0),
            Vector4::new(0.0, self.size.y(), 0.0, 1.0),
            Vector4::new(self.size.x(), 0.0, 0.0, 1.0),
            Vector4::new(self.size.x(), self.size.y(), 0.0, 1.0),
        ];

        let position = self.position - self.scaled() / 2.0;
        let scale = Matrix4::from_scale(self.scale);
        let translation =
            Matrix4::from_translation(cgmath::Vector3::new(position.x(), position.y(), 0.0));

        let rotation = get_rotation_matrix(degrees_to_radians((self.rotation * 90) as f32));

        let pre_rotation =
            Matrix4::from_translation(Vector3::new(self.size.x() / 2.0, self.size.y() / 2.0, 0.0));
        let post_rotation = Matrix4::from_translation(Vector3::new(
            -self.size.x() / 2.0,
            -self.size.y() / 2.0,
            0.0,
        ));
        let final_rotation = (pre_rotation * rotation) * post_rotation;

        let matrix = translation * scale * final_rotation;

        for vector in &mut vectors {
            (*vector) = matrix * (*vector);
        }

        let mut size = Vec2::default();
        for outer in vectors {
            for inner in vectors {
                size.set_x(max!((inner.x - outer.x).abs(), size.x()));
                size.set_y(max!((inner.y - outer.y).abs(), size.y()));
            }
        }

        let position = Vec2::new(
            min!(vectors[0].x, vectors[1].x, vectors[2].x, vectors[3].x),
            min!(vectors[0].y, vectors[1].y, vectors[2].y, vectors[3].y),
        );

        Rect::new(position, size)
    }

    pub fn flip_horizontal(&mut self, display: &Display) {
        self.horizontal_flip = !self.horizontal_flip;
        mem::swap(&mut self.texture_cords.0, &mut self.texture_cords.2);
        mem::swap(&mut self.texture_cords.1, &mut self.texture_cords.3);
        self.update_vertex_data(display);
    }

    pub fn flip_vertical(&mut self, display: &Display) {
        self.vertical_flip = !self.vertical_flip;
        mem::swap(&mut self.texture_cords.0, &mut self.texture_cords.1);
        mem::swap(&mut self.texture_cords.2, &mut self.texture_cords.3);
        self.update_vertex_data(display);
    }

    pub fn animate(&mut self, display: &Display) -> Option<Duration> {
        let frames = self.frames.read().unwrap();
        if frames.len() > 1 {
            let now = Instant::now();
            let time_passed = now.duration_since(self.last_frame);
            let delay = frames[self.index].delay;

            if time_passed > delay {
                self.index += 1;
                if self.index >= frames.len() {
                    self.index = 0;
                }

                drop(frames);
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

    pub fn crop(&mut self, cut: Rect, display: &Display) -> Option<(Vec<Image>, i32)> {
        let bounds = self.bounds();
        if !bounds.intersects(&cut) {
            return None;
        }

        let left = max!(cut.left(), bounds.left()) - bounds.x();
        let right = min!(cut.right(), bounds.right()) - bounds.x();

        let top = max!(cut.top(), bounds.top()) - bounds.y();
        let bottom = min!(cut.bottom(), bounds.bottom()) - bounds.y();

        let width = right - left;
        let height = bottom - top;

        let normalized_width = 1.0 / (bounds.width() / width);
        let normalized_height = 1.0 / (bounds.height() / height);

        let mut normalized_x = 1.0 / (bounds.width() / left);
        let mut normalized_y = 1.0 / (bounds.height() / top);

        if self.horizontal_flip {
            normalized_x = 1.0 - normalized_x - normalized_width;
        }

        if self.vertical_flip {
            normalized_y = 1.0 - normalized_y - normalized_height;
        }

        let mut old_frames = Vec::new();

        let mut frames = self.frames.write().unwrap();
        for frame in &mut *frames {
            match self.rotation {
                0 => (),
                1 => {
                    let buffer = frame.buffer().rotate270();
                    *frame.buffer_mut() = buffer;
                }
                2 => {
                    rotate180_in_place(frame.buffer_mut());
                }
                3 => {
                    let buffer = frame.buffer().rotate90();
                    *frame.buffer_mut() = buffer;
                }
                _ => unreachable!(),
            }

            let real_width = (normalized_width * frame.buffer().width() as f32) as u32;
            let real_height = (normalized_height * frame.buffer().height() as f32) as u32;

            let real_x = (normalized_x * frame.buffer().width() as f32) as u32;
            let real_y = (normalized_y * frame.buffer().height() as f32) as u32;

            let mut image = frame
                .buffer()
                .crop_imm(real_x, real_y, real_width, real_height);

            let delay = frame.delay;
            mem::swap(frame.buffer_mut(), &mut image);
            old_frames.push(Image::with_delay(image, delay));
        }
        drop(frames);

        self.update_image_data(display);
        self.update_vertex_data(display);

        let old_rotation = self.rotation;
        self.rotation = 0;
        Some((old_frames, old_rotation))
    }

    pub fn swap_frames(&mut self, frames: &mut Vec<Image>, display: &Display) {
        let mut guard = self.frames.write().unwrap();
        mem::swap(&mut *guard, frames);
        drop(guard);
        self.update_image_data(display);
        self.update_vertex_data(display);
    }

    fn update_image_data(&mut self, display: &Display) {
        let frames = self.frames.read().unwrap();
        let image = frames[self.index].buffer();
        self.size = Vec2::new(image.width() as f32, image.height() as f32);
        self.texture = get_texture(image, display);
    }

    fn update_vertex_data(&mut self, display: &Display) {
        let shape = vec![
            Vertex::new(0.0, 0.0, self.texture_cords.0.x(), self.texture_cords.0.y()),
            Vertex::new(
                0.0,
                self.size.y(),
                self.texture_cords.1.x(),
                self.texture_cords.1.y(),
            ),
            Vertex::new(
                self.size.x(),
                0.0,
                self.texture_cords.2.x(),
                self.texture_cords.2.y(),
            ),
            Vertex::new(
                self.size.x(),
                self.size.y(),
                self.texture_cords.3.x(),
                self.texture_cords.3.y(),
            ),
        ];
        self.vertices = VertexBuffer::new(display, &shape).unwrap();
    }

    pub fn rotate(&mut self, deg: i32) {
        self.rotation += deg;
        if self.rotation > 3 {
            self.rotation -= 4;
        } else if self.rotation < 0 {
            self.rotation += 4;
        }
    }
}

#[inline(always)]
fn degrees_to_radians(deg: f32) -> f32 {
    (std::f32::consts::PI / 180.0) * deg
}

fn get_rotation_matrix(rad: f32) -> Matrix4<f32> {
    #[rustfmt::skip]
    let matrix = Matrix4::new(
        rad.cos(), -(rad.sin()), 0.0, 0.0,
        rad.sin(), rad.cos(), 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );
    matrix
}

fn get_texture(image: &DynamicImage, display: &Display) -> SrgbTexture2d {
    let (width, height) = image.dimensions();

    match image {
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
        _ => {
            let data = Cow::Owned(image.to_rgba8().into_raw());
            let raw = RawImage2d {
                data,
                width,
                height,
                format: ClientFormat::U8U8U8U8,
            };
            SrgbTexture2d::with_mipmaps(display, raw, MipmapsOption::AutoGeneratedMipmaps).unwrap()
        }
    }
}
