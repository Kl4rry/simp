use cgmath::{Matrix4, Ortho, Vector3, Vector4};
use glium::{
    backend::glutin::Display,
    draw_parameters::DrawParameters,
    implement_vertex,
    index::PrimitiveType,
    program::Program,
    texture::{ClientFormat, RawImage2d, SrgbTexture2d},
    uniform,
    uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler, SamplerBehavior},
    Blend, IndexBuffer, Surface, VertexBuffer,
};
use image::{ImageBuffer, Rgba};
use std::borrow::Cow;

use super::vec2::Vec2;

macro_rules! max {
    ($x: expr) => ($x);
    ($x: expr, $($z: expr),+) => {{
        let y = max!($($z),*);
        if $x > y {
            $x
        } else {
            y
        }
    }}
}

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
    pub rotation: f32,
    shader: Program,
    vertices: VertexBuffer<Vertex>,
    indices: IndexBuffer<u8>,
    texture: SrgbTexture2d,
    sampler: SamplerBehavior,
}

impl ImageView {
    pub fn new(display: &Display, image: ImageBuffer<Rgba<u16>, Vec<u16>>) -> Self {
        let shape = vec![
            Vertex::new(0.0, 0.0, 0.0, 0.0),
            Vertex::new(0.0, image.height() as f32, 0.0, 1.0),
            Vertex::new(image.width() as f32, 0.0, 1.0, 0.0),
            Vertex::new(image.width() as f32, image.height() as f32, 1.0, 1.0),
        ];
        let index_buffer: [u8; 6] = [0, 1, 2, 2, 1, 3];

        let (width, height) = image.dimensions();
        let raw = RawImage2d {
            data: Cow::Owned(image.into_raw()),
            width,
            height,
            format: ClientFormat::U16U16U16U16,
        };
        let texture = SrgbTexture2d::new(display, raw).unwrap();

        let sampler = SamplerBehavior {
            magnify_filter: MagnifySamplerFilter::Nearest,
            minify_filter: MinifySamplerFilter::Linear,
            ..Default::default()
        };

        Self {
            size: Vec2::new(width as f32, height as f32),
            position: Vec2::default(),
            scale: 1.0,
            rotation: 0.0,
            shader: Program::from_source(
                display,
                include_str!("shader/image.vert"),
                include_str!("shader/image.frag"),
                None,
            )
            .unwrap(),
            vertices: VertexBuffer::new(display, &shape).unwrap(),
            indices: IndexBuffer::new(display, PrimitiveType::TrianglesList, &index_buffer)
                .unwrap(),
            texture,
            sampler,
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

        let rot = degrees_to_radians(self.rotation);

        #[rustfmt::skip]
        let rotation = Matrix4::new(
            rot.cos(), -(rot.sin()), 0.0, 0.0,
            rot.sin(), rot.cos(), 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        );

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

        let rot = degrees_to_radians(self.rotation);

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
}

fn degrees_to_radians(deg: f32) -> f32 {
    (std::f32::consts::PI / 180.0) * deg
}
