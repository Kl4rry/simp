use glium::glutin::{event_loop::EventLoopProxy, window::CursorIcon};
use image::io::Reader as ImageReader;
use image::{
    codecs::gif::GifDecoder, AnimationDecoder, Delay, Frame, ImageBuffer, ImageFormat, Rgb, Rgba,
};
use imagepipe::{ImageSource, Pipeline};
use psd::Psd;
use std::{
    fs,
    io::Cursor,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use user_event::UserEvent;
use usvg::{fontdb::Database, FitTo, Options, Tree};

use super::extensions::*;

pub fn load_image(proxy: EventLoopProxy<UserEvent>, path: impl AsRef<Path>) {
    let path_buf = path.as_ref().to_path_buf();
    let _ = proxy.send_event(UserEvent::SetCursor(CursorIcon::Progress));
    thread::spawn(move || {
        let start = Instant::now();
        let file = fs::read(&path_buf);
        let bytes = match file {
            Ok(bytes) => bytes,
            Err(_) => {
                let _ = proxy.send_event(UserEvent::ImageError(format!(
                    "Error could not read: {}",
                    path_buf.to_str().unwrap()
                )));
                return;
            }
        };

        let extension = path_buf
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();

        let mut loaders = [load_raw, load_svg, load_psd, load_raster];

        if RASTER.contains(&*extension) {
            loaders.swap(0, 3);
        } else if VECTOR.contains(&*extension) {
            loaders.swap(0, 1);
        } else if PHOTOSHOP.contains(&*extension) {
            loaders.swap(0, 2);
        }

        for loader in loaders {
            if let Some(image) = loader(&bytes) {
                let _ =
                    proxy.send_event(UserEvent::ImageLoaded(Some(image), Some(path_buf), start));
                return;
            }
        }

        let _ = proxy.send_event(UserEvent::ImageError(format!(
            "Error decoding image: {}",
            path_buf.to_str().unwrap()
        )));
    });
}

fn load_raster(bytes: &[u8]) -> Option<Vec<Frame>> {
    let format = match image::guess_format(&bytes) {
        Ok(format) => format,
        Err(_) => return None,
    };

    match format {
        ImageFormat::Gif => {
            if let Ok(decoder) = GifDecoder::new(bytes) {
                let frames = decoder.into_frames().collect_frames();
                if let Ok(frames) = frames {
                    return Some(frames);
                }
            }
            None
        }
        ImageFormat::WebP => {
            if let Ok(decoder) = webp_animation::Decoder::new(bytes) {
                let mut time = 0;
                let frames: Vec<Frame> = decoder
                    .into_iter()
                    .filter_map(|frame| {
                        let timestamp = frame.timestamp();
                        let difference = timestamp - time;

                        if let Ok(image) = frame.into_image() {
                            time = timestamp;
                            let delay = Delay::from_saturating_duration(Duration::from_millis(
                                difference as u64,
                            ));
                            Some(Frame::from_parts(image, 0, 0, delay))
                        } else {
                            None
                        }
                    })
                    .collect();

                if frames.is_empty() {
                    return Some(frames);
                }
            }
            if let Ok((width, height, buf)) = libwebp::WebPDecodeRGBA(bytes) {
                return Some(vec![Frame::new(
                    ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, buf.to_vec()).unwrap(),
                )]);
            }
            None
        }
        format => match ImageReader::with_format(Cursor::new(&bytes), format).decode() {
            Ok(image) => Some(vec![Frame::new(image.into_rgba8())]),
            Err(_) => None,
        },
    }
}

fn load_svg(bytes: &[u8]) -> Option<Vec<Frame>> {
    let mut fontdb = Database::new();
    fontdb.load_system_fonts();
    let options = Options {
        fontdb,
        ..Options::default()
    };

    let tree = match Tree::from_data(&bytes, &options) {
        Ok(tree) => tree,
        Err(_) => return None,
    };

    let svg = tree.svg_node();
    let mut pix_map =
        tiny_skia::Pixmap::new((*svg).size.width() as u32, (*svg).size.height() as u32).unwrap();

    resvg::render(&tree, FitTo::Original, pix_map.as_mut())?;

    let width = pix_map.width();
    let height = pix_map.height();
    let data = pix_map.take();
    Some(vec![Frame::new(
        ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, data).unwrap(),
    )])
}

fn load_psd(bytes: &[u8]) -> Option<Vec<Frame>> {
    let psd = match Psd::from_bytes(bytes) {
        Ok(psd) => psd,
        Err(_) => return None,
    };

    let raw = psd.rgba();

    Some(vec![Frame::new(
        ImageBuffer::<Rgba<u8>, _>::from_raw(psd.width(), psd.height(), raw).unwrap(),
    )])
}

fn load_raw(bytes: &[u8]) -> Option<Vec<Frame>> {
    let t = timing::Timer::start();
    let raw = match rawloader::decode(&mut Cursor::new(bytes)) {
        Ok(raw) => raw,
        Err(_) => return None,
    };

    let width = raw.width;
    let height = raw.height;
    let source = ImageSource::Raw(raw);

    let mut pipeline = match Pipeline::new_from_source(source, width, height, true) {
        Ok(pipeline) => pipeline,
        Err(_) => return None,
    };

    pipeline.run(None);
    let image = match pipeline.output_8bit(None) {
        Ok(image) => image,
        Err(_) => return None,
    };

    let image = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(
        image.width as u32,
        image.height as u32,
        image.data,
    );

    let image = match image {
        Some(image) => image,
        None => return None,
    };

    let dyn_img = image::DynamicImage::ImageRgb8(image);
    let rgba_image: ImageBuffer<Rgba<u8>, Vec<u8>> = dyn_img.into_rgba8();
    t.print_ms();
    Some(vec![Frame::new(rgba_image)])
}
