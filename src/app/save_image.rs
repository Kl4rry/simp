use std::{path::PathBuf, thread};

use glium::{glutin::event_loop::EventLoopProxy, Display};
use image_io::save::{gif, ico, jpg, png, tiff, webp, webp_animation};
use util::{Image, UserEvent};

use image::{
    imageops::{rotate180_in_place, rotate270_in, rotate90_in, flip_horizontal_in_place, flip_vertical_in_place},
    ImageBuffer,
};

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
        .add_filter("WEBP", &["webp"]);

    thread::spawn(move || {
        if let Some(path) = dialog.save_file() {
            let _ = proxy.send_event(UserEvent::Save(path));
        }
    });
}

pub fn save(proxy: EventLoopProxy<UserEvent>, mut path: PathBuf, mut frames: Vec<Image>, rotation: i32, horizontal_flip: bool, vertical_flip: bool) {
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
                    let (width, height) = frame.buffer().dimensions();
                    let mut buffer = ImageBuffer::new(height, width);
                    rotate270_in(frame.buffer(), &mut buffer).unwrap();
                    *frame.buffer_mut() = buffer;
                }
                2 => {
                    rotate180_in_place(frame.buffer_mut());
                }
                3 => {
                    let (width, height) = frame.buffer().dimensions();
                    let mut buffer = ImageBuffer::new(height, width);
                    rotate90_in(frame.buffer(), &mut buffer).unwrap();
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
            "png" => png(path, &frames[0]),
            "jpg" | "jpeg" | "jpe" | "jif" | "jfif" => jpg(path, &frames[0]),
            "ico" => ico(path, &frames[0]),
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
                png(path, &frames[0])
            }
        };

        match res {
            Err(error) => {
                let _ = proxy.send_event(UserEvent::ImageError(error.to_string()));
            }
            Ok(_) => (),
        }
    });
}
