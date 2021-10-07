use crate::vec2::Vec2;

#[derive(Clone, Copy)]
pub struct Resize {
    pub visible: bool,
    pub resample_select_index: usize,
    pub size: Vec2<i32>,
}

impl Default for Resize {
    fn default() -> Self {
        Self {
            visible: false,
            resample_select_index: 0,
            size: Vec2::default(),
        }
    }
}
