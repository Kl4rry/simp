use cgmath::Vector4;
use glium::{
    implement_vertex, index::PrimitiveType, uniform, Blend, Display, DrawParameters, IndexBuffer,
    Program, Surface, VertexBuffer,
};

use crate::{rect::Rect, vec2::Vec2};

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
}

impl Vertex {
    pub fn new(x: f32, y: f32) -> Self {
        Self { position: [x, y] }
    }
}

implement_vertex!(Vertex, position);

pub struct Crop {
    pub rect: Option<Rect>,
    pub x: String,
    pub y: String,
    pub width: String,
    pub height: String,
    pub maintain_aspect_ratio: bool,
    vertices: VertexBuffer<Vertex>,
    indices: IndexBuffer<u8>,
    shader: Box<Program>,
}

impl Crop {
    pub fn new(display: &Display) -> Self {
        let shader = Box::new(
            Program::from_source(
                display,
                include_str!("../../shader/crop.vert"),
                include_str!("../../shader/crop.frag"),
                None,
            )
            .unwrap(),
        );

        let shape = [
            Vertex::new(-1.0, 1.0),
            Vertex::new(-1.0, -1.0),
            Vertex::new(1.0, 1.0),
            Vertex::new(1.0, -1.0),
        ];
        let index_buffer = &[0, 1, 2, 2, 1, 3];

        let vertices = VertexBuffer::new(display, &shape).unwrap();
        let indices =
            IndexBuffer::new(display, PrimitiveType::TrianglesList, index_buffer).unwrap();

        Self {
            rect: None,
            x: String::new(),
            y: String::new(),
            width: String::new(),
            height: String::new(),
            maintain_aspect_ratio: false,
            vertices,
            indices,
            shader,
        }
    }

    pub fn cropping(&self) -> bool {
        self.rect.is_some()
    }

    pub fn render(
        &self,
        target: &mut glium::Frame,
        size: Vec2<f32>,
        position: Vec2<f32>,
        scale: f32,
    ) {
        if let Some(ref rect) = self.rect {
            let start = rect.position * scale + position;
            let end = (rect.position + rect.size) * scale + position;

            let start = Vector4::new(start.x(), start.y(), 0.0, 0.0).xy();
            let end = Vector4::new(end.x(), end.y(), 0.0, 0.0).xy();

            let start = Vec2::new(start.x, start.y);
            let end = Vec2::new(end.x, end.y);

            target
                .draw(
                    &self.vertices,
                    &self.indices,
                    &self.shader,
                    &uniform! { start: start, end: end, size: *size },
                    &DrawParameters {
                        blend: Blend::alpha_blending(),
                        ..DrawParameters::default()
                    },
                )
                .unwrap();
        }
    }
}
