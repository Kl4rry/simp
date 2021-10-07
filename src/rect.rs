use crate::vec2::Vec2;

#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub struct Rect {
    pub position: Vec2<f32>,
    pub size: Vec2<f32>,
}

impl Rect {
    #[inline]
    pub fn new(position: Vec2<f32>, size: Vec2<f32>) -> Self {
        Rect { position, size }
    }

    #[inline]
    pub fn x(&self) -> f32 {
        self.position.x()
    }

    #[inline]
    pub fn y(&self) -> f32 {
        self.position.y()
    }

    #[inline]
    pub fn width(&self) -> f32 {
        self.size.x()
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.size.y()
    }

    #[inline]
    pub fn left(&self) -> f32 {
        self.position.x()
    }

    #[inline]
    pub fn right(&self) -> f32 {
        self.position.x() + self.size.x()
    }

    #[inline]
    pub fn top(&self) -> f32 {
        self.position.y()
    }

    #[inline]
    pub fn bottom(&self) -> f32 {
        self.position.y() + self.size.y()
    }

    #[inline]
    #[rustfmt::skip]
    pub fn intersects(&self, rect: &Self) -> bool {
        let x  = ((self.x() + self.width() / 2.0) - (rect.x() + rect.width() / 2.0)).abs() * 2.0 < (self.width() + rect.width());
        let y = ((self.y() + self.height() / 2.0) - (rect.y() + rect.height() / 2.0)).abs() * 2.0 < (self.height() + rect.height());
        x && y
    }
}
