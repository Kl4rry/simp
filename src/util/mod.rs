use std::{path::PathBuf, time::Duration};

use image::{Delay, DynamicImage, Frame, ImageBuffer, Rgba};

use crate::app::op_queue::Output;

pub mod extensions;
pub mod matrix;

#[macro_export]
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

#[macro_export]
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

#[derive(Clone, Debug)]
pub struct Image {
    pub image: DynamicImage,
    pub delay: Duration,
}

impl Image {
    pub fn new(image: DynamicImage) -> Self {
        Image {
            image,
            delay: Duration::default(),
        }
    }

    pub fn with_delay(image: DynamicImage, delay: Duration) -> Self {
        Image { image, delay }
    }

    pub fn buffer(&self) -> &DynamicImage {
        &self.image
    }

    pub fn buffer_mut(&mut self) -> &mut DynamicImage {
        &mut self.image
    }
}

impl From<ImageBuffer<Rgba<u8>, Vec<u8>>> for Image {
    fn from(buffer: ImageBuffer<Rgba<u8>, Vec<u8>>) -> Self {
        Image {
            image: DynamicImage::ImageRgba8(buffer),
            delay: Duration::default(),
        }
    }
}

impl From<Frame> for Image {
    fn from(frame: Frame) -> Self {
        let (num, deno) = frame.delay().numer_denom_ms();
        let delay = Duration::from_millis((num / deno) as u64);
        let buffer = frame.into_buffer();
        Image {
            image: DynamicImage::ImageRgba8(buffer),
            delay,
        }
    }
}

impl From<Image> for Frame {
    fn from(image: Image) -> Frame {
        let duration = image.delay;
        let frame = image.image.to_rgba8();
        Frame::from_parts(frame, 0, 0, Delay::from_saturating_duration(duration))
    }
}

pub enum UserEvent {
    ErrorMessage(String),
    QueueLoad(PathBuf),
    QueueSave(PathBuf),
    QueueDelete(PathBuf),
    Output(Option<Output>),
    LoadBytes(Vec<u8>),
    RepaintRequest(egui::RequestRepaintInfo),
    Wake,
    Exit,
}

#[derive(Debug, Clone)]
pub struct ImageData {
    pub frames: Vec<Image>,
    pub metadata: Vec<(String, String)>,
}

impl ImageData {
    pub fn new(frames: Vec<Image>, metadata: Vec<(String, String)>) -> Self {
        Self { frames, metadata }
    }
}

impl From<Vec<Image>> for ImageData {
    fn from(frames: Vec<Image>) -> Self {
        Self {
            frames,
            metadata: Vec::new(),
        }
    }
}

pub fn p2(v: impl Into<mint::Point2<f32>>) -> mint::Point2<f32> {
    v.into()
}

pub fn v2(v: impl Into<mint::Vector2<f32>>) -> mint::Vector2<f32> {
    v.into()
}
