use std::{
    mem,
    path::PathBuf,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use cgmath::{Deg, Matrix4, Ortho, Vector2, Vector3, Vector4};
use image::GenericImageView;
use num_traits::Zero;
use winit::event_loop::EventLoopProxy;

use self::mosaic::Mosaic;
use super::op_queue::{Output, UserEventLoopProxyExt};
use crate::{
    WgpuState, max,
    rect::Rect,
    util::{Image, ImageData, UserEvent, matrix::OPENGL_TO_WGPU_MATRIX},
};

pub mod crop_renderer;
pub mod image_renderer;
pub mod mosaic;

mod crop;
use crop::Crop;

mod texture;

pub struct ImageView {
    pub mosaic: Vec<Mosaic>,
    pub size: Vector2<f32>,
    pub position: Vector2<f32>,
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
    pub playing: bool,
}

impl ImageView {
    pub fn new(wgpu: &WgpuState, image_data: Arc<ImageData>, path: Option<PathBuf>) -> Self {
        let frames = &image_data.frames;
        let image = frames[0].buffer();
        let (width, height) = image.dimensions();
        let mosaic = Mosaic::from_images(wgpu, image_data.clone());

        Self {
            mosaic,
            size: Vector2::new(width as f32, height as f32),
            position: Vector2::zero(),
            scale: 1.0,
            rotation: 0,
            image_data: Arc::new(RwLock::new(image_data)),
            last_frame: Instant::now(),
            index: 0,
            horizontal_flip: false,
            vertical_flip: false,
            crop: Crop::new(),
            path,
            hue: 0.0,
            contrast: 0.0,
            brightness: 0.0,
            saturation: 0.0,
            grayscale: false,
            invert: false,
            playing: true,
        }
    }

    pub fn get_rotation_mat(&self) -> Matrix4<f32> {
        let rotation = Matrix4::from_angle_z(Deg((self.rotation * 90) as f32));
        let pre_rotation =
            Matrix4::from_translation(Vector3::new(self.size.x / 2.0, self.size.y / 2.0, 0.0));
        let post_rotation =
            Matrix4::from_translation(Vector3::new(-self.size.x / 2.0, -self.size.y / 2.0, 0.0));
        (pre_rotation * rotation) * post_rotation
    }

    pub fn get_flip_mat(&self) -> Matrix4<f32> {
        let hori = if self.horizontal_flip { -1.0 } else { 1.0 };

        let vert = if self.vertical_flip { -1.0 } else { 1.0 };

        let flip = Matrix4::from_nonuniform_scale(hori, vert, 1.0);
        let pre_rotation =
            Matrix4::from_translation(Vector3::new(self.size.x / 2.0, self.size.y / 2.0, 0.0));
        let post_rotation =
            Matrix4::from_translation(Vector3::new(-self.size.x / 2.0, -self.size.y / 2.0, 0.0));
        (pre_rotation * flip) * post_rotation
    }

    pub fn get_uniform(&self, size: Vector2<f32>) -> image_renderer::Uniform {
        let ortho: Matrix4<f32> = Ortho {
            left: 0.0,
            right: size.x,
            bottom: size.y,
            top: 0.0,
            near: 0.0,
            far: 1.0,
        }
        .into();

        let position = self.position - self.scaled() / 2.0;
        let scale = Matrix4::from_scale(self.scale);
        let translation = Matrix4::from_translation(Vector3::new(position.x, position.y, 0.0));
        let flip = self.get_flip_mat();
        let rotation = self.get_rotation_mat();
        let matrix = OPENGL_TO_WGPU_MATRIX * ortho * translation * scale * flip * rotation;

        image_renderer::Uniform {
            matrix,
            size: Vector2::new(size.x, size.y),
            hue: self.hue,
            contrast: self.contrast,
            brightness: self.brightness,
            saturation: self.saturation,
            grayscale: self.grayscale as u32,
            invert: self.invert as u32,
        }
    }

    pub fn scaled(&self) -> Vector2<f32> {
        self.size * self.scale
    }

    pub fn real_size(&self) -> Vector2<f32> {
        let mut vectors = vec![
            Vector4::new(0.0, 0.0, 0.0, 1.0),
            Vector4::new(0.0, self.size.y, 0.0, 1.0),
            Vector4::new(self.size.x, 0.0, 0.0, 1.0),
            Vector4::new(self.size.x, self.size.y, 0.0, 1.0),
        ];

        let rotation = Matrix4::from_angle_z(Deg((self.rotation * 90) as f32));

        let scale = Matrix4::from_scale(self.scale);

        let matrix = scale * rotation;

        for vector in &mut vectors {
            (*vector) = matrix * (*vector);
        }

        let mut size = Vector2::new(0.0, 0.0);

        for outer in &vectors {
            for inner in &vectors {
                size.x = max!((inner.x - outer.x).abs(), size.x);
                size.y = max!((inner.y - outer.y).abs(), size.y);
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

    pub fn animate(&mut self) -> Duration {
        let guard = self.image_data.read().unwrap();
        let frames = &guard.frames;
        if !self.playing || frames.len() <= 1 {
            return Duration::MAX;
        }

        let now = Instant::now();
        let time_passed = now.duration_since(self.last_frame);
        let delay = frames[self.index].delay;

        if time_passed > delay {
            self.index += 1;
            if self.index >= frames.len() {
                self.index = 0;
            }

            self.last_frame = now;

            delay
        } else {
            delay - time_passed
        }
    }

    pub fn len_frames(&mut self) -> usize {
        let guard = self.image_data.read().unwrap();
        guard.frames.len()
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
        self.mosaic = Mosaic::from_images(wgpu, self.image_data.read().unwrap().clone());
        self.size = Vector2::new(width as f32, height as f32);
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

    pub fn rotated_size(&self) -> Vector2<f32> {
        let mut size = self.size;
        if self.rotation % 2 != 0 {
            mem::swap(&mut size.x, &mut size.y);
        }
        size
    }

    pub fn cropping(&self) -> bool {
        self.crop.cropping()
    }

    pub fn start_crop(&mut self) {
        let size = self.rotated_size();
        let position = Vector2::new(0.0, 0.0);
        self.crop.rect = Some(Rect { position, size });
        self.crop.x = position.x.to_string();
        self.crop.y = position.y.to_string();
        self.crop.width = size.x.to_string();
        self.crop.height = size.y.to_string();
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
            self.position += Vector2::from((vec2.x, vec2.y));
        }
    }
}
