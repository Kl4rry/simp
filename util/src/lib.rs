use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use glium::glutin::window::CursorIcon;
use image::{Delay, Frame, ImageBuffer, Rgba};

pub mod extensions;

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
    pub img: ImageBuffer<Rgba<u8>, Vec<u8>>,
    pub delay: Duration,
}

impl Image {
    #[inline]
    pub fn new(image: ImageBuffer<Rgba<u8>, Vec<u8>>) -> Self {
        Image {
            img: image,
            delay: Duration::default(),
        }
    }

    #[inline]
    pub fn with_delay(image: ImageBuffer<Rgba<u8>, Vec<u8>>, delay: Duration) -> Self {
        Image { img: image, delay }
    }

    pub fn buffer(&self) -> &ImageBuffer<Rgba<u8>, Vec<u8>> {
        &self.img
    }

    pub fn buffer_mut(&mut self) -> &mut ImageBuffer<Rgba<u8>, Vec<u8>> {
        &mut self.img
    }
}

impl From<Frame> for Image {
    #[inline]
    fn from(frame: Frame) -> Self {
        let (num, deno) = frame.delay().numer_denom_ms();
        let delay = Duration::from_millis((num / deno) as u64);
        Image {
            img: frame.into_buffer(),
            delay,
        }
    }
}

impl From<Image> for Frame {
    fn from(image: Image) -> Frame {
        let duration = image.delay;
        Frame::from_parts(image.img, 0, 0, Delay::from_saturating_duration(duration))
    }
}

pub enum UserEvent {
    ImageLoaded(Option<Vec<Image>>, Option<PathBuf>, Instant),
    ImageError(String),
    SetCursor(CursorIcon),
    Save(PathBuf),
}
