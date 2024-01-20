use std::{
    mem,
    path::PathBuf,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use cgmath::{Deg, Matrix4, Ortho, Vector3, Vector4};
use image::GenericImageView;
use wgpu::util::DeviceExt;
use winit::event_loop::EventLoopProxy;

use super::op_queue::{Output, UserEventLoopProxyExt};
use crate::{
    max,
    rect::Rect,
    util::{matrix::OPENGL_TO_WGPU_MATRIX, Image, ImageData, UserEvent},
    vec2::Vec2,
    WgpuState,
};

pub mod renderer;

mod crop;
use crop::Crop;

mod texture;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    pub fn new(x: f32, y: f32, tex_x: f32, tex_y: f32) -> Self {
        Self {
            position: [x, y],
            tex_coords: [tex_x, tex_y],
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

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
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    texture: texture::Texture,
}

impl ImageView {
    pub fn new(wgpu: &WgpuState, image_data: Arc<ImageData>, path: Option<PathBuf>) -> Self {
        let frames = &image_data.frames;
        let image = frames[0].buffer();
        let (width, height) = image.dimensions();
        let texture = texture::Texture::from_image(&wgpu.device, &wgpu.queue, image, None);

        let indices: &[u32] = &[0, 1, 2, 2, 1, 3];
        let indices = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Image Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

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
            crop: Crop::new(wgpu),
            vertices: get_vertex_buffer(wgpu, width as f32, height as f32),
            indices,
            texture,
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

    pub fn get_uniforms(&self, size: Vec2<f32>) -> renderer::Uniform {
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
        let matrix = OPENGL_TO_WGPU_MATRIX * ortho * translation * scale * rotation;

        renderer::Uniform {
            matrix,
            flip_horizontal: self.horizontal_flip as u32,
            flip_vertical: self.vertical_flip as u32,
            size,
            padding: Default::default(),
            hue: self.hue,
            contrast: self.contrast,
            brightness: self.brightness,
            saturation: self.saturation,
            grayscale: self.grayscale as u32,
            invert: self.invert as u32,
        }
    }

    pub fn get_buffers(&self) -> (&wgpu::Buffer, &wgpu::Buffer) {
        (&self.vertices, &self.indices)
    }

    pub fn get_texture(&self) -> &texture::Texture {
        &self.texture
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

    pub fn animate(&mut self, wgpu: &WgpuState) -> Duration {
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
                self.update_image_data(wgpu);

                self.last_frame = now;

                delay
            } else {
                delay - time_passed
            }
        } else {
            Duration::MAX
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

    pub fn swap_frames(&mut self, wgpu: &WgpuState, frames: &mut Vec<Image>) {
        let mut guard = self.image_data.write().unwrap();
        mem::swap(&mut Arc::make_mut(&mut *guard).frames, frames);
        let (width, height) = guard.frames[0].buffer().dimensions();
        drop(guard);
        self.update_image_data(wgpu);
        self.vertices = get_vertex_buffer(wgpu, width as f32, height as f32);
    }

    fn update_image_data(&mut self, wgpu: &WgpuState) {
        let guard = self.image_data.read().unwrap();
        let frames = &guard.frames;
        let image = frames[self.index].buffer();
        self.size = Vec2::new(image.width() as f32, image.height() as f32);
        self.texture = texture::Texture::from_image(&wgpu.device, &wgpu.queue, image, None);
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

fn get_vertex_buffer(wgpu: &WgpuState, width: f32, height: f32) -> wgpu::Buffer {
    let texture_cords = (
        Vec2::new(1.0, 1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, 0.0),
    );
    let shape = [
        Vertex::new(0.0, 0.0, texture_cords.0.x(), texture_cords.0.y()),
        Vertex::new(0.0, height, texture_cords.1.x(), texture_cords.1.y()),
        Vertex::new(width, 0.0, texture_cords.2.x(), texture_cords.2.y()),
        Vertex::new(width, height, texture_cords.3.x(), texture_cords.3.y()),
    ];

    wgpu.device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Image Vertex Buffer"),
            contents: bytemuck::cast_slice(shape.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        })
}
