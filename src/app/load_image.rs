use glium::glutin::event_loop::EventLoopProxy;
use image::io::Reader as ImageReader;
use image::{ImageBuffer, ImageFormat, Rgba, Frame, codecs::gif::GifDecoder, AnimationDecoder};
use libwebp::WebPDecodeRGBA;
use psd::Psd;
use std::{fs, io::Cursor, path::Path, thread, time::Instant};
use usvg::{fontdb::Database, FitTo, Options, Tree};

use super::super::UserEvent;

pub fn load_image(proxy: EventLoopProxy<UserEvent>, path: impl AsRef<Path>) {
    let path_buf = path.as_ref().to_path_buf();
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

        if let Some(image) = load_raster(&bytes) {
            let _ = proxy.send_event(UserEvent::ImageLoaded(image, path_buf, start));
            return;
        }
        
        if let Some(image) = load_svg(&bytes) {
            let _ = proxy.send_event(UserEvent::ImageLoaded(vec![Frame::new(image)], path_buf, start));
            return;
        }

        if let Some(image) = load_psd(&bytes) {
            let _ = proxy.send_event(UserEvent::ImageLoaded(vec![Frame::new(image)], path_buf, start));
            return;
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
        },
        ImageFormat::WebP => {
            match WebPDecodeRGBA(bytes) {
                Ok((width, height, buf)) => {
                    if let Some(image) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, buf.to_vec()) {
                        Some(vec![Frame::new(image)])
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        },
        format => {
            match ImageReader::with_format(Cursor::new(&bytes), format).decode() {
                Ok(image) => Some(vec![Frame::new(image.into_rgba8())]),
                Err(_) => None,
            }
        },
    }
}

fn load_svg(bytes: &[u8]) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
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
    Some(ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, data).unwrap())
}

fn load_psd(bytes: &[u8]) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let psd = match Psd::from_bytes(bytes) {
        Ok(psd) => psd,
        Err(_) => return None,
    };

    let raw = psd.flatten_layers_rgba(&|(_, _)| return true).unwrap();
    Some(ImageBuffer::<Rgba<u8>, _>::from_raw(psd.width(), psd.height(), raw).unwrap())
}
