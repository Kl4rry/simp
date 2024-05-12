use std::{
    error, fmt,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use image::{
    codecs::{gif::GifEncoder, jpeg::JpegEncoder},
    Frame, GenericImageView, ImageError, ImageFormat,
};
use webp_animation::prelude::*;

use crate::util::Image;

type SaveResult<T> = Result<T, SaveError>;

#[derive(Debug)]
pub enum SaveError {
    Image(ImageError),
    Io(std::io::Error),
    #[allow(unused)]
    WebpAnimation(webp_animation::Error),
    LibWebp(libwebp::error::WebPSimpleError),
}

impl fmt::Display for SaveError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SaveError::Image(ref e) => e.fmt(f),
            SaveError::Io(ref e) => e.fmt(f),
            SaveError::WebpAnimation(_) => write!(f, "error encoding webp"),
            SaveError::LibWebp(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for SaveError {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            SaveError::Image(ref e) => Some(e),
            SaveError::Io(ref e) => Some(e),
            SaveError::WebpAnimation(_) => None,
            SaveError::LibWebp(ref e) => Some(e),
        }
    }
}

impl From<ImageError> for SaveError {
    #[inline]
    fn from(err: ImageError) -> SaveError {
        SaveError::Image(err)
    }
}

impl From<std::io::Error> for SaveError {
    #[inline]
    fn from(err: std::io::Error) -> SaveError {
        SaveError::Io(err)
    }
}

impl From<webp_animation::Error> for SaveError {
    #[inline]
    fn from(err: webp_animation::Error) -> SaveError {
        SaveError::WebpAnimation(err)
    }
}

impl From<libwebp::error::WebPSimpleError> for SaveError {
    #[inline]
    fn from(err: libwebp::error::WebPSimpleError) -> SaveError {
        SaveError::LibWebp(err)
    }
}

fn open_file(path: impl AsRef<Path>) -> Result<File, std::io::Error> {
    OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)
}

fn get_temp_path(path: impl AsRef<Path>) -> PathBuf {
    let mut id = String::from('.');
    id.push_str(&nanoid::nanoid!(6));
    let mut buf = path.as_ref().to_path_buf();
    buf.set_file_name(id);
    buf
}

#[inline]
pub fn save_with_format(
    path: impl AsRef<Path>,
    image: &Image,
    format: ImageFormat,
) -> SaveResult<()> {
    let temp_path = get_temp_path(path.as_ref());
    let file = open_file(&temp_path)?;

    if let Err(err) = image.buffer().write_to(&mut BufWriter::new(file), format) {
        let _ = fs::remove_file(&temp_path);
        Err(err)?;
    }

    if let Err(err) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        Err(err)?;
    }

    Ok(())
}

#[inline]
pub fn jpeg(path: impl AsRef<Path>, image: &Image, quality: u8) -> SaveResult<()> {
    let temp_path = get_temp_path(path.as_ref());
    let file = open_file(&temp_path)?;

    let mut encoder = JpegEncoder::new_with_quality(BufWriter::new(file), quality);
    let buffer = image.buffer();

    encoder.encode(
        buffer.as_bytes(),
        buffer.width(),
        buffer.height(),
        buffer.color().into(),
    )?;

    Ok(fs::rename(temp_path, path)?)
}

#[inline]
pub fn gif(path: impl AsRef<Path>, images: Vec<Image>) -> SaveResult<()> {
    let temp_path = get_temp_path(path.as_ref());
    let file = open_file(&temp_path)?;

    let frames: Vec<Frame> = images.into_iter().map(|image| image.into()).collect();
    let mut encoder = GifEncoder::new(BufWriter::new(file));
    encoder.encode_frames(frames)?;

    Ok(fs::rename(temp_path, path)?)
}

#[inline]
pub fn webp_animation(
    path: impl AsRef<Path>,
    images: Vec<Image>,
    quality: f32,
    lossy: bool,
) -> SaveResult<()> {
    let encoding_type = if lossy {
        EncodingType::Lossy(LossyEncodingConfig::default())
    } else {
        EncodingType::Lossless
    };
    let config = EncodingConfig {
        encoding_type,
        quality,
        method: 6,
    };

    let dimensions = images[0].buffer().dimensions();
    let options = EncoderOptions {
        encoding_config: Some(config),
        ..Default::default()
    };
    let mut encoder = Encoder::new_with_options(dimensions, options)?;
    let mut timestamp: i32 = 0;
    for image in images {
        encoder.add_frame(&image.buffer().to_rgba8().into_raw(), timestamp)?;
        timestamp += image.delay.as_millis() as i32;
    }

    let webp_data = encoder.finalize(timestamp)?;

    let temp_path = get_temp_path(path.as_ref());
    let mut file = open_file(&temp_path)?;
    file.write_all(&webp_data)?;

    Ok(fs::rename(temp_path, path)?)
}

#[inline]
pub fn webp(path: impl AsRef<Path>, image: &Image, quality: f32, lossy: bool) -> SaveResult<()> {
    let (width, height) = image.buffer().dimensions();

    let webp_data = if lossy {
        libwebp::WebPEncodeLosslessRGBA(
            &image.buffer().to_rgba8().into_raw(),
            width,
            height,
            width * 4,
        )?
    } else {
        libwebp::WebPEncodeRGBA(
            &image.buffer().to_rgba8().into_raw(),
            width,
            height,
            width * 4,
            quality,
        )?
    };

    let temp_path = get_temp_path(path.as_ref());
    let mut file = open_file(&temp_path)?;
    file.write_all(&webp_data)?;

    Ok(fs::rename(temp_path, path)?)
}
