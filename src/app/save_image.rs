use std::{path::PathBuf, thread};

use glium::{glutin::event_loop::EventLoopProxy, Display};
use image::{
    imageops::{flip_horizontal_in_place, flip_vertical_in_place, rotate180_in_place},
    ImageOutputFormat,
};
use image_io::save::{gif, save_with_format, tiff, webp, webp_animation, farbfeld};
use util::{Image, UserEvent};

pub fn open(name: String, proxy: EventLoopProxy<UserEvent>, display: &Display) {
    let dialog = rfd::FileDialog::new()
        .set_file_name(&name)
        .set_parent(display.gl_window().window())
        .add_filter("PNG", &["png"])
        .add_filter("JPEG", &["jpg", "jpeg", "jpe", "jif", "jfif"])
        .add_filter("GIF", &["gif"])
        .add_filter("ICO", &["ico"])
        .add_filter("BMP", &["bmp"])
        .add_filter("TIFF", &["tiff", "tif"])
        .add_filter("WEBP", &["webp"])
        .add_filter("Farbfeld", &["ff", "farbfeld"])
        .add_filter("TGA", &["tga"]);

    thread::spawn(move || {
        if let Some(path) = dialog.save_file() {
            let _ = proxy.send_event(UserEvent::Save(path));
        }
    });
}

pub fn save(
    proxy: EventLoopProxy<UserEvent>,
    mut path: PathBuf,
    mut frames: Vec<Image>,
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
        for frame in &mut frames {
            match rotation {
                0 => (),
                1 => {
                    let buffer = frame.buffer().rotate270();
                    *frame.buffer_mut() = buffer;
                }
                2 => {
                    rotate180_in_place(frame.buffer_mut());
                }
                3 => {
                    let buffer = frame.buffer().rotate90();
                    *frame.buffer_mut() = buffer;
                }
                _ => unreachable!(),
            }

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
            _ => {
                path.set_extension("png");
                save_with_format(path, &frames[0], ImageOutputFormat::Png)
            }
        };

        if let Err(error) = res {
            let _ = proxy.send_event(UserEvent::ImageError(error.to_string()));
        }
    });
}
