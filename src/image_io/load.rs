use std::{io::Cursor, time::Duration};

use fontdb::Database;
use image::{
    codecs::{gif::GifDecoder, openexr::OpenExrDecoder, png::PngDecoder},
    io::Reader as ImageReader,
    AnimationDecoder, DynamicImage, Frame, ImageBuffer, ImageFormat, Rgb, Rgba,
};
use imagepipe::{ImageSource, Pipeline};
use psd::Psd;
use resvg::usvg_text_layout::TreeTextToPath;
use usvg::{FitTo, Options, Tree};

use crate::util::Image;

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
            if let Ok(decoder) = GifDecoder::new(bytes) {
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
                return Some(vec![Image::from(
                    ImageBuffer::from_raw(width, height, buf.to_vec()).unwrap(),
                )]);
            }
            None
        }
        ImageFormat::Png => {
            if let Ok(decoder) = PngDecoder::new(bytes) {
                if decoder.is_apng() {
                    return Some(decode_images(decoder.apng().into_frames()));
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

    let Ok(mut tree) = Tree::from_data(bytes, &options) else {
        return None;
    };
    tree.convert_text(&fontdb, true);

    let size = tree.size.to_screen_size();
    let mut pix_map = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();

    resvg::render(
        &tree,
        FitTo::Original,
        tiny_skia::Transform::identity(),
        pix_map.as_mut(),
    )?;

    let width = pix_map.width();
    let height = pix_map.height();
    let data = pix_map.take();
    Some(vec![Image::from(
        ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, data).unwrap(),
    )])
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
    let Ok(image) = pipeline.output_8bit(None) else {
        return None;
    };

    let image = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(
        image.width as u32,
        image.height as u32,
        image.data,
    );

    let dyn_img = image::DynamicImage::ImageRgb8(image?);
    let rgba_image: ImageBuffer<Rgba<u8>, Vec<u8>> = dyn_img.into_rgba8();
    Some(vec![Image::from(rgba_image)])
}
