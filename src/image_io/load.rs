use std::{io::Cursor, time::Duration};

use fontdb::Database;
use image::{
    codecs::{gif::GifDecoder, openexr::OpenExrDecoder, png::PngDecoder},
    io::Reader as ImageReader,
    AnimationDecoder, DynamicImage, Frame, ImageBuffer, ImageFormat, Rgb, RgbImage, Rgba,
    RgbaImage,
};
use imagepipe::{ImageSource, Pipeline};
use psd::Psd;
use resvg::usvg::{Options, Tree};

use crate::{app::preferences::PREFERENCES, util::Image};

pub fn decode_images<T, E>(frames: T) -> Vec<Image>
where
    T: IntoIterator<Item = Result<Frame, E>>,
{
    frames
        .into_iter()
        .filter_map(|result| match result {
            Ok(frame) => Some(frame.into()),
            Err(_) => None,
        })
        .collect()
}

pub fn load_raster(bytes: &[u8]) -> Option<Vec<Image>> {
    let format = match image::guess_format(bytes) {
        Ok(format) => format,
        Err(_) => return None,
    };

    match format {
        ImageFormat::Gif => {
            if let Ok(decoder) = GifDecoder::new(Cursor::new(bytes)) {
                return Some(decode_images(decoder.into_frames()));
            }
            None
        }
        ImageFormat::WebP => {
            if let Ok(decoder) = webp_animation::Decoder::new(bytes) {
                let mut time = 0;
                let frames: Vec<Image> = decoder
                    .into_iter()
                    .filter_map(|frame| {
                        let timestamp = frame.timestamp();
                        let difference = timestamp - time;

                        let (width, height) = frame.dimensions();
                        let data = frame.data().to_vec();

                        ImageBuffer::from_raw(width, height, data).map(|image| {
                            time = timestamp;
                            let delay = Duration::from_millis(difference as u64);
                            Image::with_delay(DynamicImage::ImageRgba8(image), delay)
                        })
                    })
                    .collect();

                if !frames.is_empty() {
                    return Some(frames);
                }
            }
            if let Ok((width, height, buf)) = libwebp::WebPDecodeRGBA(bytes) {
                return Some(vec![Image::from(ImageBuffer::from_raw(
                    width,
                    height,
                    buf.to_vec(),
                )?)]);
            }
            None
        }
        ImageFormat::Png => {
            if let Ok(decoder) = PngDecoder::new(Cursor::new(bytes)) {
                if decoder.is_apng().unwrap_or(false) {
                    return Some(decode_images(decoder.apng().ok()?.into_frames()));
                } else if let Ok(image) = DynamicImage::from_decoder(decoder) {
                    return Some(vec![Image::new(image)]);
                }
            }
            None
        }
        ImageFormat::OpenExr => {
            if let Ok(decoder) = OpenExrDecoder::new(Cursor::new(bytes)) {
                if let Ok(mut image) = DynamicImage::from_decoder(decoder) {
                    // For some reason some of the values are NaN which cannot be converted to srgb to be sent to the GPU.
                    // This simply sets them to zero and hopes for the best.
                    match &mut image {
                        DynamicImage::ImageRgb32F(buffer) => {
                            for sample in buffer.as_flat_samples_mut().as_mut_slice() {
                                if sample.is_nan() {
                                    *sample = 0.0;
                                }
                            }
                        }
                        DynamicImage::ImageRgba32F(buffer) => {
                            for sample in buffer.as_flat_samples_mut().as_mut_slice() {
                                if sample.is_nan() {
                                    *sample = 0.0;
                                }
                            }
                        }
                        _ => (),
                    }
                    return Some(vec![Image::new(image)]);
                }
            }
            None
        }
        format => {
            let mut reader = ImageReader::with_format(Cursor::new(&bytes), format);
            reader.no_limits();
            match reader.decode() {
                Ok(image) => Some(vec![Image::new(image)]),
                Err(_) => None,
            }
        }
    }
}

pub fn load_un_detectable_raster(bytes: &[u8]) -> Option<Vec<Image>> {
    match ImageReader::with_format(Cursor::new(&bytes), ImageFormat::Tga).decode() {
        Ok(image) => Some(vec![Image::new(image)]),
        Err(_) => None,
    }
}

pub fn load_svg(bytes: &[u8]) -> Option<Vec<Image>> {
    let mut fontdb = Database::new();
    fontdb.load_system_fonts();
    let options = Options::default();

    let tree = Tree::from_data(bytes, &options, &fontdb).ok()?;
    let size = tree.size().to_int_size();

    let min_size = PREFERENCES.lock().unwrap().min_svg_size.max(100);
    let smaller_axis = size.width().min(size.height());
    let size = if smaller_axis < min_size {
        let scale_factor = min_size as f32 / smaller_axis as f32;
        size.scale_by(scale_factor).unwrap_or(size)
    } else {
        size
    };

    let transform = resvg::tiny_skia::Transform::from_scale(
        size.width() as f32 / tree.size().width(),
        size.height() as f32 / tree.size().height(),
    );

    let mut pix_map = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(&tree, transform, &mut pix_map.as_mut());

    let width = pix_map.width();
    let height = pix_map.height();
    let data = pix_map.take();
    Some(vec![Image::from(ImageBuffer::<Rgba<u8>, _>::from_raw(
        width, height, data,
    )?)])
}

pub fn load_psd(bytes: &[u8]) -> Option<Vec<Image>> {
    let psd = match Psd::from_bytes(bytes) {
        Ok(psd) => psd,
        Err(_) => return None,
    };

    let raw = psd.rgba();

    Some(vec![Image::from(
        ImageBuffer::<Rgba<u8>, _>::from_raw(psd.width(), psd.height(), raw).unwrap(),
    )])
}

pub fn load_raw(bytes: &[u8]) -> Option<Vec<Image>> {
    let Ok(raw) = rawloader::decode(&mut Cursor::new(bytes)) else {
        return None;
    };

    let source = ImageSource::Raw(raw);
    let Ok(mut pipeline) = Pipeline::new_from_source(source) else {
        return None;
    };

    pipeline.run(None);
    let Ok(image) = pipeline.output_16bit(None) else {
        return None;
    };

    let image = ImageBuffer::<Rgb<u16>, Vec<u16>>::from_raw(
        image.width as u32,
        image.height as u32,
        image.data,
    )?;

    let image = image::DynamicImage::ImageRgb16(image);
    Some(vec![Image::new(image)])
}

pub fn load_jxl(bytes: &[u8]) -> Option<Vec<Image>> {
    #[cfg(feature = "jxl")]
    {
        use jpegxl_rs::image::ToDynamic;
        let decoder = jpegxl_rs::decoder_builder()
            .unpremul_alpha(true)
            .build()
            .ok()?;
        let image = decoder.decode_to_image(&bytes).ok()??;
        Some(vec![Image::new(image)])
    }

    #[cfg(not(feature = "jxl"))]
    {
        let _ = bytes;
        None
    }
}

pub fn load_heif(bytes: &[u8]) -> Option<Vec<Image>> {
    #[cfg(feature = "heif")]
    {
        use libheif_rs::{ColorSpace, DecodingOptions, HeifContext, LibHeif, RgbChroma};
        let lib_heif = LibHeif::new();
        let ctx = HeifContext::read_from_bytes(bytes).ok()?;
        let handle = ctx.primary_image_handle().ok()?;
        let image = lib_heif
            .decode(
                &handle,
                ColorSpace::Rgb(RgbChroma::Rgba),
                DecodingOptions::new().map(|mut o| {
                    o.set_convert_hdr_to_8bit(true);
                    o
                }),
            )
            .ok()?;
        let planes = image.planes();
        let interleaved_plane = planes.interleaved?;
        let raw = interleaved_plane.data.to_vec();
        let rgba_image =
            RgbaImage::from_raw(interleaved_plane.width, interleaved_plane.height, raw)?;
        Some(vec![Image::new(DynamicImage::ImageRgba8(rgba_image))])
    }

    #[cfg(not(feature = "heif"))]
    {
        let _ = bytes;
        None
    }
}

pub fn load_qoi(bytes: &[u8]) -> Option<Vec<Image>> {
    let (
        qoi::Header {
            width,
            height,
            channels,
            //colorspace, FIXME handle linear color
            ..
        },
        decoded,
    ) = qoi::decode_to_vec(bytes).ok()?;
    let image = match channels {
        qoi::Channels::Rgb => DynamicImage::ImageRgb8(RgbImage::from_raw(width, height, decoded)?),
        qoi::Channels::Rgba => {
            DynamicImage::ImageRgba8(RgbaImage::from_raw(width, height, decoded)?)
        }
    };
    Some(vec![Image::new(image)])
}
