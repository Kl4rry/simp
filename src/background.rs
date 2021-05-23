use glium::{
    backend::glutin::Display,
    VertexBuffer,
    IndexBuffer,
    index::PrimitiveType,
    program::Program,
    Surface, uniform, implement_vertex,
};

use super::vec2::Vec2;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);

pub struct Background {
    shader: Program,
    vertices: VertexBuffer<Vertex>,
    indices: IndexBuffer<u8>,
}

impl Background {
    pub fn new(display: &Display) -> Self {
        let shape = vec![Vertex { position: [-1.0, 1.0] }, Vertex { position: [ -1.0,  -1.0] }, Vertex { position: [ 1.0, 1.0] }, Vertex { position: [ 1.0, -1.0] }];
        let index_buffer: [u8; 6] = [0, 1, 2, 2, 1, 3];

        Self {
            shader: Program::from_source(display, include_str!("shader/background.vert"), include_str!("shader/background.frag"), None).unwrap(),
            vertices: VertexBuffer::new(display, &shape).unwrap(),
            indices: IndexBuffer::new(display, PrimitiveType::TrianglesList, &index_buffer).unwrap(),
        }
    }

    pub fn render(&self, target: &mut glium::Frame, size: Vec2<f32>) {
        target.draw(&self.vertices, &self.indices, &self.shader, &uniform! { size: *size }, &Default::default()).unwrap();
    }
}

