use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
    thread,
};

use glium::{glutin::event_loop::EventLoopProxy, Display};
use image::{
    imageops::{flip_horizontal_in_place, flip_vertical_in_place},
    ImageOutputFormat,
};

use super::op_queue::{Output, UserEventLoopProxyExt};
use crate::{
    image_io::save::{exr, farbfeld, gif, save_with_format, tiff, webp, webp_animation},
    util::{Image, ImageData, UserEvent},
};

pub fn open(name: String, proxy: EventLoopProxy<UserEvent>, display: &Display) {
    let dialog = rfd::FileDialog::new()
        .set_file_name(&name)
        .set_parent(display.gl_window().window())
        .add_filter("PNG", &["png", "apng"])
        .add_filter("JPEG", &["jpg", "jpeg", "jpe", "jif", "jfif"])
        .add_filter("GIF", &["gif"])
        .add_filter("ICO", &["ico"])
        .add_filter("BMP", &["bmp"])
        .add_filter("TIFF", &["tiff", "tif"])
        .add_filter("WEBP", &["webp"])
        .add_filter("Targaformat", &["ff", "farbfeld"])
        .add_filter("TGA", &["tga"])
        .add_filter("EXR", &["exr"]);

    thread::spawn(move || {
        if let Some(path) = dialog.save_file() {
            let _ = proxy.send_event(UserEvent::QueueSave(path));
        }
    });
}

pub fn save(
    proxy: EventLoopProxy<UserEvent>,
    mut path: PathBuf,
    image_data: Arc<RwLock<ImageData>>,
    rotation: i32,
    horizontal_flip: bool,
    vertical_flip: bool,
) {
    let os_str = path.extension();
    let ext = match os_str {
        Some(ext) => ext.to_string_lossy().to_string().to_lowercase(),
        None => String::from("png"),
    };
    path.set_extension(&ext);

    thread::spawn(move || {
        let guard = image_data.read().unwrap();
        let old_frames = &guard.frames;
        let mut frames = Vec::new();
        for frame in old_frames {
            let buffer = match rotation {
                0 => frame.buffer().clone(),
                1 => frame.buffer().rotate270(),
                2 => frame.buffer().rotate180(),
                3 => frame.buffer().rotate90(),
                _ => unreachable!("image is rotated more then 360 degrees"),
            };
            frames.push(Image::with_delay(buffer, frame.delay));
        }

        for frame in frames.iter_mut() {
            if horizontal_flip {
                flip_horizontal_in_place(frame.buffer_mut());
            }

            if vertical_flip {
                flip_vertical_in_place(frame.buffer_mut());
            }
        }

        let res = match ext.as_str() {
            "png" => save_with_format(path, &frames[0], ImageOutputFormat::Png),
            "jpg" | "jpeg" | "jpe" | "jif" | "jfif" => {
                save_with_format(path, &frames[0], ImageOutputFormat::Jpeg(100))
            }
            "ico" => save_with_format(path, &frames[0], ImageOutputFormat::Ico),
            "tga" => save_with_format(path, &frames[0], ImageOutputFormat::Tga),
            "ff" | "farbfeld" => farbfeld(path, &frames[0]),
            "tiff" | "tif" => tiff(path, &frames[0]),
            "gif" => gif(path, frames),
            "webp" => {
                if frames.len() > 1 {
                    webp_animation(path, frames)
                } else {
                    webp(path, &frames[0])
                }
            }
            "exr" => exr(path, &frames[0]),
            _ => {
                path.set_extension("png");
                save_with_format(path, &frames[0], ImageOutputFormat::Png)
            }
        };

        proxy.send_output(Output::Done);
        if let Err(error) = res {
            let _ = proxy.send_event(UserEvent::ErrorMessage(error.to_string()));
        }
    });
}
