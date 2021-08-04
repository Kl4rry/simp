use std::{
    fs::{rename, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use image::{
    codecs::{
        bmp::BmpEncoder, gif::GifEncoder, ico::IcoEncoder, jpeg::JpegEncoder, png::PngEncoder,
        tiff::TiffEncoder,
    },
    ColorType::Rgba8,
    EncodableLayout, Frame,
};
use libwebp::WebPEncodeLosslessRGBA;
use util::Image;
use uuid::Uuid;
use webp_animation::{Encoder, EncoderOptions, EncodingConfig};

fn open_file(path: impl AsRef<Path>) -> Result<File, std::io::Error> {
    OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)
}

fn get_temp_path(path: impl AsRef<Path>) -> PathBuf {
    let uuid = Uuid::new_v4().to_string();
    let mut buf = path.as_ref().to_path_buf();
    buf.set_file_name(uuid);
    buf
}

pub fn png(path: impl AsRef<Path>, image: &Image) -> Result<(), std::io::Error> {
    let temp_path = get_temp_path(path.as_ref());
    let file = open_file(&temp_path)?;

    let encoder = PngEncoder::new(file);
    let buffer = image.buffer();
    encoder
        .encode(buffer.as_bytes(), buffer.width(), buffer.height(), Rgba8)
        .unwrap();

    rename(temp_path, path)
}

pub fn jpg(path: impl AsRef<Path>, image: &Image) -> Result<(), std::io::Error> {
    let temp_path = get_temp_path(path.as_ref());
    let mut file = open_file(&temp_path)?;

    let mut encoder = JpegEncoder::new(&mut file);
    encoder.encode_image(image.buffer()).unwrap();

    rename(temp_path, path)
}

pub fn gif(path: impl AsRef<Path>, images: Vec<Image>) -> Result<(), std::io::Error> {
    let temp_path = get_temp_path(path.as_ref());
    let file = open_file(&temp_path)?;

    let frames: Vec<Frame> = images.into_iter().map(|image| image.into()).collect();
    let mut encoder = GifEncoder::new(file);
    encoder.encode_frames(frames).unwrap();

    rename(temp_path, path)
}

pub fn ico(path: impl AsRef<Path>, image: &Image) -> Result<(), std::io::Error> {
    let temp_path = get_temp_path(path.as_ref());
    let file = open_file(&temp_path)?;

    let encoder = IcoEncoder::new(file);
    let buffer = image.buffer();
    encoder
        .encode(buffer.as_bytes(), buffer.width(), buffer.height(), Rgba8)
        .unwrap();

    rename(temp_path, path)
}

pub fn bmp(path: impl AsRef<Path>, image: &Image) -> Result<(), std::io::Error> {
    let temp_path = get_temp_path(path.as_ref());
    let mut file = open_file(&temp_path)?;

    let mut encoder = BmpEncoder::new(&mut file);
    let buffer = image.buffer();
    encoder
        .encode(buffer.as_bytes(), buffer.width(), buffer.height(), Rgba8)
        .unwrap();

    rename(temp_path, path)
}

pub fn tiff(path: impl AsRef<Path>, image: &Image) -> Result<(), std::io::Error> {
    let temp_path = get_temp_path(path.as_ref());
    let file = open_file(&temp_path)?;

    let encoder = TiffEncoder::new(file);
    let buffer = image.buffer();
    encoder
        .encode(buffer.as_bytes(), buffer.width(), buffer.height(), Rgba8)
        .unwrap();

    rename(temp_path, path)
}

pub fn webp_animation(path: impl AsRef<Path>, images: Vec<Image>) -> Result<(), std::io::Error> {
    let config = EncodingConfig {
        encoding_type: webp_animation::prelude::EncodingType::Lossless,
        quality: 100.0,
        method: 6,
    };
    let dimensions = images[0].buffer().dimensions();
    let mut options = EncoderOptions::default();
    options.encoding_config = Some(config);
    let mut encoder = Encoder::new_with_options(dimensions, options).unwrap();
    let mut timestamp: i32 = 0;
    for image in images {
        encoder
            .add_frame(image.buffer().as_bytes(), timestamp)
            .unwrap();
        timestamp += image.delay.as_millis() as i32;
    }

    let webp_data = encoder.finalize(timestamp).unwrap();

    let temp_path = get_temp_path(path.as_ref());
    let mut file = open_file(&temp_path)?;
    file.write(&*webp_data)?;

    rename(temp_path, path)
}

pub fn webp(path: impl AsRef<Path>, image: &Image) -> Result<(), std::io::Error> {
    let (width, height) = image.buffer().dimensions();
    let webp_data =
        WebPEncodeLosslessRGBA(image.buffer().as_bytes(), width, height, width * 4).unwrap();

    let temp_path = get_temp_path(path.as_ref());
    let mut file = open_file(&temp_path)?;
    file.write(&*webp_data)?;

    rename(temp_path, path)
}
