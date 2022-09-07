use std::{
    error, fmt, fs,
    path::{Path, PathBuf},
    thread,
};

use glium::{glutin::event_loop::EventLoopProxy, Display};
use rexif::ExifTag;

use crate::{
    image_io::load::*,
    util::{extensions::*, ImageData, UserEvent},
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

pub fn open(proxy: EventLoopProxy<UserEvent>, display: &Display, folder: bool) {
    let dialog = rfd::FileDialog::new().set_parent(display.gl_window().window());
    thread::spawn(move || {
        let pick = if folder {
            dialog.pick_folder()
        } else {
            dialog.pick_file()
        };
        if let Some(file) = pick {
            let _ = proxy.send_event(UserEvent::QueueLoad(file));
        }
    });
}

pub fn load_uncached(path: impl AsRef<Path>) -> Result<ImageData, LoadError> {
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

    let mut metadata = Vec::new();
    if let Ok(exif) = rexif::parse_buffer_quiet(&bytes).0 {
        for entry in exif.entries {
            if ExifTag::UnknownToMe != entry.tag {
                let text = entry.value_more_readable;
                metadata.push((entry.tag.to_string(), text.to_string()));
            }
        }
    }

    for loader in loaders {
        if let Some(image) = loader(&bytes) {
            return Ok(ImageData::new(image, metadata));
        }
    }
    Err(LoadError::Decoding(path_buf))
}
