use glium::glutin::{event_loop::EventLoopProxy, window::CursorIcon};
use image_loader::*;
use std::{fs, path::Path, thread, time::Instant};
use util::UserEvent;

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
