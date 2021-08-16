use std::{
    error, fmt, fs,
    path::{Path, PathBuf},
    thread,
    time::Instant,
};

use glium::{
    glutin::{event_loop::EventLoopProxy, window::CursorIcon},
    Display,
};
use image_io::load::*;
use util::{extensions::*, Image, UserEvent};

use crate::app::Cache;

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Decoding(PathBuf),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LoadError::Io(ref e) => e.fmt(f),
            LoadError::Decoding(ref path_buf) => {
                write!(f, "error decoding image: {:?}", path_buf.to_string_lossy())
            }
        }
    }
}

impl error::Error for LoadError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            LoadError::Io(ref e) => Some(e),
            LoadError::Decoding(_) => None,
        }
    }
}

impl From<std::io::Error> for LoadError {
    fn from(err: std::io::Error) -> LoadError {
        LoadError::Io(err)
    }
}

pub fn open(proxy: EventLoopProxy<UserEvent>, display: &Display, cache: Cache) {
    let dialog = rfd::FileDialog::new().set_parent(display.gl_window().window());
    thread::spawn(move || {
        if let Some(file) = dialog.pick_file() {
            load(proxy, file, cache);
        }
    });
}

pub fn load(proxy: EventLoopProxy<UserEvent>, path: impl AsRef<Path>, cache: Cache) {
    let path_buf = path.as_ref().to_path_buf();
    let _ = proxy.send_event(UserEvent::SetCursor(CursorIcon::Progress));
    thread::spawn(move || {
        let start = Instant::now();

        // make sure lock is dropped after cache lookup
        {
            if let Some(images) = cache.lock().unwrap().get(&path_buf) {
                let images = images.clone();
                let _ =
                    proxy.send_event(UserEvent::ImageLoaded(Some(images), Some(path_buf), start));
                return;
            }
        }

        match load_uncached(&path_buf) {
            Ok(images) => {
                cache.lock().unwrap().put(path_buf.clone(), images.clone());
                let _ =
                    proxy.send_event(UserEvent::ImageLoaded(Some(images), Some(path_buf), start));
            }
            Err(error) => {
                let _ = proxy.send_event(UserEvent::Error(error.to_string()));
            }
        }
    });
}

pub fn load_uncached(path: impl AsRef<Path>) -> Result<Vec<Image>, LoadError> {
    let path_buf = path.as_ref().to_path_buf();
    let bytes = fs::read(&path_buf)?;

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
            return Ok(image);
        }
    }
    Err(LoadError::Decoding(path_buf))
}
