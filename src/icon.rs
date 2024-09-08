use std::io::Cursor;

use image::{ImageFormat, ImageReader};
use winit::window::Icon;

pub fn get_icon() -> Icon {
    let bytes = include_bytes!("../icon.ico");
    let image = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Ico)
        .decode()
        .unwrap()
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_vec(), width, height).unwrap()
}
