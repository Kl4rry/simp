use std::{fs, path::Path, thread, time::Instant};

use glium::{
    glutin::{event_loop::EventLoopProxy, window::CursorIcon},
    Display,
};
use image_io::load::*;
use util::{extensions::*, UserEvent};

pub fn open(proxy: EventLoopProxy<UserEvent>, display: &Display) {
    let dialog = rfd::FileDialog::new().set_parent(display.gl_window().window());
    thread::spawn(move || {
        if let Some(file) = dialog.pick_file() {
            load(proxy, file);
        }
    });
}

pub fn load(proxy: EventLoopProxy<UserEvent>, path: impl AsRef<Path>) {
    let path_buf = path.as_ref().to_path_buf();
    let _ = proxy.send_event(UserEvent::SetCursor(CursorIcon::Progress));
    thread::spawn(move || {
        let start = Instant::now();
        let file = fs::read(&path_buf);
        let bytes = match file {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = proxy.send_event(UserEvent::ImageError(error.to_string()));
                return;
            }
        };

        let extension = path_buf
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        let mut loaders = [
            load_raw,
            load_svg,
            load_psd,
            load_raster,
            load_un_detectable_raster,
        ];

        if RASTER.contains(&*extension) {
            loaders.swap(0, 3);
        } else if VECTOR.contains(&*extension) {
            loaders.swap(0, 1);
        } else if PHOTOSHOP.contains(&*extension) {
            loaders.swap(0, 2);
        } else if UNDETECTABLE_RASTER.contains(&*extension) {
            loaders.swap(0, 4);
        }

        for loader in loaders {
            if let Some(image) = loader(&bytes) {
                let _ =
                    proxy.send_event(UserEvent::ImageLoaded(Some(image), Some(path_buf), start));
                return;
            }
        }

        let _ = proxy.send_event(UserEvent::ImageError(format!(
            "Error decoding image: {:?}",
            path_buf.to_str().unwrap()
        )));
    });
}
