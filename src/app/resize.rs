use crate::vec2::Vec2;

#[derive(Clone, Copy, Default)]
pub struct Resize {
    pub visible: bool,
    pub resample_select_index: usize,
    pub size: Vec2<i32>,
}
