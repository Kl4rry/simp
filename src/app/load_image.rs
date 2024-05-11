use std::{
    error, fmt, fs,
    path::{Path, PathBuf},
    thread,
};

use rexif::ExifTag;
use winit::event_loop::EventLoopProxy;

use crate::{
    image_io::load::*,
    util::{extensions::*, ImageData, UserEvent},
    WgpuState,
};

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Decoding(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LoadError::Io(ref e) => e.fmt(f),
            LoadError::Decoding(ref source) => {
                write!(f, "error decoding image: {:?}", source)
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

pub fn open(proxy: EventLoopProxy<UserEvent>, wgpu: &WgpuState, folder: bool) {
    let ext: Vec<_> = EXTENSIONS.iter().copied().collect();
    let raw: Vec<_> = RAW.iter().copied().collect();
    let dialog = rfd::FileDialog::new()
        .set_parent(&wgpu.window)
        .add_filter("All", &ext)
        .add_filter("png", &["png", "apng"])
        .add_filter("svg", &["svg"])
        .add_filter("jpeg", &["jpg", "jpeg", "jpe", "jif", "jfif"])
        .add_filter("Photoshop", &["psd"])
        .add_filter("Animated", &["apng", "gif", "webp"])
        .add_filter("Raw", &raw);
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
    load_from_bytes(&bytes, Some(path_buf))
}

pub fn load_from_bytes(bytes: &[u8], path_buf: Option<PathBuf>) -> Result<ImageData, LoadError> {
    let source = path_buf
        .as_ref()
        .map(|path| path.to_string_lossy().into())
        .unwrap_or_else(|| String::from("from stdin"));
    let extension = path_buf
        .unwrap_or_default()
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
        load_jxl,
        load_heif,
        load_qoi,
    ];

    if QOI.contains(&extension.as_str()) {
        loaders.swap(0, 7);
    }
    if HEIF.contains(&extension.as_str()) {
        loaders.swap(0, 6);
    } else if JXL.contains(&extension.as_str()) {
        loaders.swap(0, 5);
    } else if RASTER.contains(&extension.as_str()) {
        loaders.swap(0, 3);
    } else if VECTOR.contains(&extension.as_str()) {
        loaders.swap(0, 1);
    } else if PHOTOSHOP.contains(&extension.as_str()) {
        loaders.swap(0, 2);
    } else if UNDETECTABLE_RASTER.contains(&extension.as_str()) {
        loaders.swap(0, 4);
    }

    let mut metadata = Vec::new();
    if let Ok(exif) = rexif::parse_buffer_quiet(bytes).0 {
        for entry in exif.entries {
            if ExifTag::UnknownToMe != entry.tag {
                let text = entry.value_more_readable;
                metadata.push((entry.tag.to_string(), text.to_string()));
            }
        }
    }

    for loader in loaders {
        if let Some(image) = loader(bytes) {
            return Ok(ImageData::new(image, metadata));
        }
    }

    Err(LoadError::Decoding(source))
}
