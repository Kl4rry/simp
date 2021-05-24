use glium::glutin::window::Icon;
use image::{io::Reader as ImageReader};
use std::io::Cursor;

pub fn get_icon() -> Icon {
    let bytes = include_bytes!("../icon.ico");
    let image = ImageReader::with_format(Cursor::new(bytes), image::guess_format(bytes).unwrap())
        .decode()
        .unwrap()
        .into_rgba8();
    
    Icon::from_rgba(image.to_vec(), image.width(), image.height()).unwrap()
}