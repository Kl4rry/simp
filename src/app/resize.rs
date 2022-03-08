use image::imageops::FilterType;

use crate::vec2::Vec2;

#[derive(Clone)]
pub struct Resize {
    pub visible: bool,
    pub resample: FilterType,
    pub width: String,
    pub height: String,
    pub maintain_aspect_ratio: bool,
}

impl Default for Resize {
    fn default() -> Self {
        Self {
            visible: false,
            resample: FilterType::Nearest,
            width: String::from("0"),
            height: String::from("0"),
            maintain_aspect_ratio: true,
        }
    }
}

impl Resize {
    pub fn set_size(&mut self, size: Vec2<u32>) {
        self.width = size.x().to_string();
        self.height = size.y().to_string();
    }
}
