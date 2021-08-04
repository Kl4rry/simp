use std::{
    fs::{rename, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use image::{
    codecs::{gif::GifEncoder, tiff::TiffEncoder},
    Frame, GenericImageView, ImageOutputFormat,
    error::ImageResult
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

pub fn save_with_format(
    path: impl AsRef<Path>,
    image: &Image,
    format: ImageOutputFormat,
) -> ImageResult<()> {
    let temp_path = get_temp_path(path.as_ref());
    let mut file = open_file(&temp_path)?;
    image.buffer().write_to(&mut file, format)?;

    Ok(rename(temp_path, path)?)
}

pub fn tiff(path: impl AsRef<Path>, image: &Image) -> ImageResult<()> {
    let temp_path = get_temp_path(path.as_ref());
    let file = open_file(&temp_path)?;

    let encoder = TiffEncoder::new(file);
    let buffer = image.buffer();

    encoder
        .encode(
            buffer.as_bytes(),
            buffer.width(),
            buffer.height(),
            buffer.color(),
        )?;

    Ok(rename(temp_path, path)?)
}

pub fn gif(path: impl AsRef<Path>, images: Vec<Image>) -> ImageResult<()> {
    let temp_path = get_temp_path(path.as_ref());
    let file = open_file(&temp_path)?;

    let frames: Vec<Frame> = images.into_iter().map(|image| image.into()).collect();
    let mut encoder = GifEncoder::new(file);
    encoder.encode_frames(frames)?;

    Ok(rename(temp_path, path)?)
}

pub fn webp_animation(path: impl AsRef<Path>, images: Vec<Image>) -> ImageResult<()> {
    let config = EncodingConfig {
        encoding_type: webp_animation::prelude::EncodingType::Lossless,
        quality: 100.0,
        method: 6,
    };
    let dimensions = images[0].buffer().dimensions();
    let options = EncoderOptions { encoding_config: Some(config), ..Default::default() };
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
    file.write_all(&*webp_data)?;

    Ok(rename(temp_path, path)?)
}

pub fn webp(path: impl AsRef<Path>, image: &Image) -> ImageResult<()> {
    let (width, height) = image.buffer().dimensions();
    let webp_data =
        WebPEncodeLosslessRGBA(image.buffer().as_bytes(), width, height, width * 4).unwrap();

    let temp_path = get_temp_path(path.as_ref());
    let mut file = open_file(&temp_path)?;
    file.write_all(&*webp_data)?;

    Ok(rename(temp_path, path)?)
}
