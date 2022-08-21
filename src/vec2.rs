use std::{
    cmp::{Eq, PartialEq},
    convert::From,
    ops::{Add, AddAssign, Deref, DerefMut, Div, Mul, Sub, SubAssign},
};

use glium::uniforms::{AsUniformValue, UniformValue};

use crate::max;

#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub struct Vec2<T> {
    inner: [T; 2],
}

impl<T: Copy> Vec2<T> {
    #[inline]
    pub fn new(x: T, y: T) -> Self {
        Self { inner: [x, y] }
    }

    pub fn splat(size: T) -> Self {
        Self {
            inner: [size, size],
        }
    }

    #[inline]
    pub fn x(&self) -> T {
        self.inner[0]
    }

    #[inline]
    pub fn y(&self) -> T {
        self.inner[1]
    }

    #[inline]
    pub fn mut_x(&mut self) -> &mut T {
        &mut self.inner[0]
    }

    #[inline]
    pub fn mut_y(&mut self) -> &mut T {
        &mut self.inner[1]
    }

    #[inline]
    pub fn set_x(&mut self, x: T) {
        self.inner[0] = x;
    }

    #[inline]
    pub fn set_y(&mut self, y: T) {
        self.inner[1] = y;
    }

    #[inline]
    pub fn swap(&mut self) {
        let x = self.inner[0];
        let y = self.inner[1];
        self.inner[0] = y;
        self.inner[1] = x;
    }

    pub fn map<U, F>(self, f: F) -> Vec2<U>
    where
        U: Copy,
        F: Copy + FnOnce(T) -> U,
    {
        Vec2::new(f(self.inner[0]), f(self.inner[1]))
    }
}

impl<T: Copy + num_traits::Float> Vec2<T> {
    #[inline]
    pub fn round(self) -> Self {
        Self::new(self.inner[0].round(), self.inner[1].round())
    }

    #[inline]
    pub fn floor(self) -> Self {
        Self::new(self.inner[0].floor(), self.inner[1].floor())
    }
}

impl<T: Copy + std::cmp::PartialOrd> Vec2<T> {
    #[inline]
    pub fn max(&self, x: T, y: T) -> Self {
        Self {
            inner: [max!(self.inner[0], x), max!(self.inner[1], y)],
        }
    }
}

impl Vec2<f32> {
    #[inline]
    pub fn length(&self) -> f32 {
        (self.inner[0] * self.inner[0] + self.inner[1] * self.inner[1]).sqrt()
    }
}

impl Vec2<f64> {
    #[inline]
    pub fn length(&self) -> f64 {
        (self.inner[0] * self.inner[0] + self.inner[1] * self.inner[1]).sqrt()
    }
}

impl<T: Add + Add<Output = T> + Copy> Add for Vec2<T> {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            inner: [
                self.inner[0] + other.inner[0],
                self.inner[1] + other.inner[1],
            ],
        }
    }
}

impl<T: Add + Add<Output = T> + Copy> AddAssign for Vec2<T> {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            inner: [self[0] + other[0], self[1] + other[1]],
        };
    }
}

impl<T: Sub + Sub<Output = T> + Copy> Sub for Vec2<T> {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            inner: [
                self.inner[0] - other.inner[0],
                self.inner[1] - other.inner[1],
            ],
        }
    }
}

impl<T: Sub + Sub<Output = T> + Copy> SubAssign for Vec2<T> {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            inner: [self[0] - other[0], self[1] - other[1]],
        };
    }
}

impl<T: Mul + Mul<Output = T> + Copy> Mul<T> for Vec2<T> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: T) -> Self {
        Self {
            inner: [self.inner[0] * rhs, self.inner[1] * rhs],
        }
    }
}

impl<T: Div + Div<Output = T> + Copy> Div<T> for Vec2<T> {
    type Output = Self;
    #[inline]
    fn div(self, rhs: T) -> Self {
        Self {
            inner: [self.inner[0] / rhs, self.inner[1] / rhs],
        }
    }
}

impl<T: Eq> Eq for Vec2<T> {}

impl<T> From<[T; 2]> for Vec2<T> {
    #[inline]
    fn from(inner: [T; 2]) -> Self {
        Self { inner }
    }
}

impl<T> From<(T, T)> for Vec2<T> {
    #[inline]
    fn from(inner: (T, T)) -> Self {
        Self {
            inner: [inner.0, inner.1],
        }
    }
}

impl From<Vec2<f32>> for egui::Pos2 {
    fn from(vec2: Vec2<f32>) -> Self {
        Self {
            x: vec2.x(),
            y: vec2.y(),
        }
    }
}

impl From<Vec2<f32>> for egui::Vec2 {
    fn from(vec2: Vec2<f32>) -> Self {
        Self {
            x: vec2.x(),
            y: vec2.y(),
        }
    }
}

impl From<egui::Vec2> for Vec2<f32> {
    fn from(vec2: egui::Vec2) -> Self {
        Self::new(vec2.x, vec2.y)
    }
}

impl From<egui::Pos2> for Vec2<f32> {
    fn from(vec2: egui::Pos2) -> Self {
        Self::new(vec2.x, vec2.y)
    }
}

impl<T> Deref for Vec2<T> {
    type Target = [T; 2];
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Vec2<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl AsUniformValue for Vec2<f32> {
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Vec2(self.inner)
    }
}
