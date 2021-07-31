use glium::glutin::window::CursorIcon;
use image::{DynamicImage, Frame, ImageBuffer, Rgba};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub struct Image {
    pub img: DynamicImage,
    pub delay: Duration,
}

impl Image {
    #[inline]
    pub fn new(image: ImageBuffer<Rgba<u8>, Vec<u8>>) -> Self {
        Image {
            img: DynamicImage::ImageRgba8(image),
            delay: Duration::default(),
        }
    }

    #[inline]
    pub fn with_delay(image: ImageBuffer<Rgba<u8>, Vec<u8>>, delay: Duration) -> Self {
        Image {
            img: DynamicImage::ImageRgba8(image),
            delay,
        }
    }

    pub fn buffer(&self) -> &ImageBuffer<Rgba<u8>, Vec<u8>> {
        self.img.as_rgba8().unwrap()
    }
}

impl From<Frame> for Image {
    #[inline]
    fn from(frame: Frame) -> Self {
        let (num, deno) = frame.delay().numer_denom_ms();
        let delay = Duration::from_millis((num / deno) as u64);
        Image {
            img: DynamicImage::ImageRgba8(frame.into_buffer()),
            delay,
        }
    }
}

#[inline(always)]
pub fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b {
        a
    } else {
        b
    }
}

#[inline(always)]
pub fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b {
        a
    } else {
        b
    }
}


pub enum UserEvent {
    ImageLoaded(Option<Vec<Image>>, Option<PathBuf>, Instant),
    ImageError(String),
    SetCursor(CursorIcon),
}
