use std::{fs::OpenOptions, path::PathBuf};

use glium::{glutin::event_loop::EventLoopProxy, Display};
use image::{codecs::png::PngEncoder, ColorType::Rgba8, EncodableLayout};
use util::{Image, UserEvent};

pub fn open(name: &str, frames: &Vec<Image>, proxy: EventLoopProxy<UserEvent>, display: &Display) {
    let dialog = rfd::FileDialog::new()
        .set_file_name(name)
        .set_parent(display.gl_window().window())
        .add_filter("png", &["png", "PNG"])
        .add_filter("jpg", &["jpg", ".jpeg", ".jpe", ".jif", ".jfif"])
        .save_file();

    if let Some(path) = dialog {
        save(proxy, path, frames.clone());
    }
}

pub fn save(proxy: EventLoopProxy<UserEvent>, mut path: PathBuf, frames: Vec<Image>) {
    let os_str = path.extension();
    let ext = match os_str {
        Some(ext) => ext.to_string_lossy().to_string().to_lowercase(),
        None => String::from("png"),
    };
    path.set_extension(&ext);

    let file = match OpenOptions::new().write(true).create(true).open(path) {
        Ok(file) => file,
        Err(error) => {
            let _ = proxy.send_event(UserEvent::ImageError(error.to_string()));
            return;
        }
    };

    let res = match ext.as_str() {
        "png" => {
            let encoder = PngEncoder::new(file);
            let buffer = frames[0].buffer();
            encoder.encode(buffer.as_bytes(), buffer.width(), buffer.height(), Rgba8)
        }
        _ => {
            path.set_extension("png");
            let encoder = PngEncoder::new(file);
            let buffer = frames[0].buffer();
            encoder.encode(buffer.as_bytes(), buffer.width(), buffer.height(), Rgba8)
        },
    };

    if let Err(error) = res {
        let _ = proxy.send_event(UserEvent::ImageError(error.to_string()));
    }
}
