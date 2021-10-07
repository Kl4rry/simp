use std::{
    error, fmt, fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    thread,
};

use glium::{
    glutin::{event_loop::EventLoopProxy, window::CursorIcon},
    Display,
};

use crate::{
    app::{cache::Cache, image_loader::ImageLoader},
    image_io::load::*,
    util::{extensions::*, Image, UserEvent},
};

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

pub fn open(
    proxy: EventLoopProxy<UserEvent>,
    display: &Display,
    cache: Arc<Cache>,
    loading: Arc<RwLock<ImageLoader>>,
) {
    let dialog = rfd::FileDialog::new().set_parent(display.gl_window().window());
    thread::spawn(move || {
        if let Some(file) = dialog.pick_file() {
            load(proxy, file, cache, loading);
        }
    });
}

pub fn prefetch(
    proxy: EventLoopProxy<UserEvent>,
    path: impl AsRef<Path>,
    cache: Arc<Cache>,
    loading: Arc<RwLock<ImageLoader>>,
) {
    let path_buf = path.as_ref().to_path_buf();

    if cache.contains(&path_buf) {
        return;
    }

    {
        let mut guard = loading.write().unwrap();
        if guard.loading.contains(&path_buf) {
            return;
        } else {
            guard.loading.insert(path_buf.clone());
        }
    }

    thread::spawn(move || {
        match load_uncached(&path_buf) {
            Ok(images) => {
                let images = Arc::new(RwLock::new(images));
                cache.put(path_buf.clone(), images.clone());
                let _ = proxy.send_event(UserEvent::ImageLoaded(images, Some(path_buf)));
            }
            Err(error) => {
                let _ = proxy.send_event(UserEvent::LoadError(error.to_string(), path_buf));
            }
        };
    });
}

pub fn load(
    proxy: EventLoopProxy<UserEvent>,
    path: impl AsRef<Path>,
    cache: Arc<Cache>,
    loading: Arc<RwLock<ImageLoader>>,
) {
    let path_buf = path.as_ref().to_path_buf();

    {
        let mut guard = loading.write().unwrap();
        guard.target_file = Some(path_buf.clone());
        if guard.loading.contains(&path_buf) {
            return;
        } else {
            guard.loading.insert(path_buf.clone());
        }
    }

    if let Some(images) = cache.get(&path_buf) {
        let _ = proxy.send_event(UserEvent::ImageLoaded(images, Some(path_buf)));
        return;
    }

    thread::spawn(move || {
        let _ = proxy.send_event(UserEvent::SetCursor(CursorIcon::Progress));

        match load_uncached(&path_buf) {
            Ok(images) => {
                let images = Arc::new(RwLock::new(images));
                cache.put(path_buf.clone(), images.clone());
                let _ = proxy.send_event(UserEvent::ImageLoaded(images, Some(path_buf)));
            }
            Err(error) => {
                let _ = proxy.send_event(UserEvent::LoadError(error.to_string(), path_buf));
            }
        };
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
