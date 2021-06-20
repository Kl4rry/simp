use cgmath::{Matrix4, Ortho};
use glium::{
    backend::glutin::Display, draw_parameters::DrawParameters, implement_vertex,
    index::PrimitiveType, program::Program, uniform, IndexBuffer, Surface, VertexBuffer,
};

use super::super::Vec2;

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
    pub inner: Option<Inner>,
    pub cropping: bool,
    shader: Program,
}

pub struct Inner {
    pub start: Vec2<f32>,
    pub current: Vec2<f32>,
}

impl Crop {
    pub fn new(display: &Display) -> Self {
        let shader = Program::from_source(
            display,
            include_str!("../shader/crop.vert"),
            include_str!("../shader/crop.frag"),
            None,
        )
        .unwrap();

        Self {
            inner: None,
            cropping: false,
            shader,
        }
    }

    pub fn start() {}

    pub fn render(&self, target: &mut glium::Frame, display: &Display, size: Vec2<f32>) {
        if let Some(ref inner) = self.inner {
            let ortho: Matrix4<f32> = Ortho {
                left: 0.0,
                right: size.x(),
                bottom: size.y(),
                top: 0.0,
                near: 0.0,
                far: 1.0,
            }
            .into();
            let raw: [[f32; 4]; 4] = ortho.into();

            let shape = [
                Vertex::new(inner.start.x(), inner.start.y()),
                Vertex::new(inner.start.x(), inner.current.y()),
                Vertex::new(inner.current.x(), inner.start.y()),
                Vertex::new(inner.current.x(), inner.current.y()),
            ];
            let index_buffer: &[u8] = &[0, 1, 0, 2, 1, 3, 2, 3];

            let vertices = VertexBuffer::new(display, &shape).unwrap();
            let indices =
                IndexBuffer::new(display, PrimitiveType::LinesList, index_buffer).unwrap();

            target
                .draw(
                    &vertices,
                    &indices,
                    &self.shader,
                    &uniform! { matrix: raw },
                    &DrawParameters {
                        line_width: Some(3.0),
                        ..DrawParameters::default()
                    },
                )
                .unwrap();
        }
    }
}
