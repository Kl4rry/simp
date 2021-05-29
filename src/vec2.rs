use std::{
    cmp::{Eq, PartialEq},
    convert::From,
    ops::{Add, AddAssign, Deref, DerefMut, Div, Mul, Sub, SubAssign},
};

#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub struct Vec2<T> {
    inner: [T; 2],
}

impl<T: Copy> Vec2<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { inner: [x, y] }
    }

    pub fn x(&self) -> T {
        self.inner[0]
    }

    pub fn y(&self) -> T {
        self.inner[1]
    }

    pub fn mut_x(&mut self) -> &mut T {
        &mut self.inner[0]
    }

    pub fn mut_y(&mut self) -> &mut T {
        &mut self.inner[1]
    }

    pub fn set_x(&mut self, x: T) {
        self.inner[0] = x;
    }

    pub fn set_y(&mut self, y: T) {
        self.inner[1] = y;
    }
}

impl Vec2<f32> {
    #[allow(dead_code)]
    pub fn length(&self) -> f32 {
        (self.inner[0] * self.inner[0] + self.inner[1] * self.inner[1]).sqrt()
    }
}

impl Vec2<f64> {
    #[allow(dead_code)]
    pub fn length(&self) -> f64 {
        (self.inner[0] * self.inner[0] + self.inner[1] * self.inner[1]).sqrt()
    }
}

impl<T: Add + Add<Output = T> + Copy> Add for Vec2<T> {
    type Output = Self;

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
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            inner: [self[0] + other[0], self[1] + other[1]],
        };
    }
}

impl<T: Sub + Sub<Output = T> + Copy> Sub for Vec2<T> {
    type Output = Self;

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
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            inner: [self[0] - other[0], self[1] - other[1]],
        };
    }
}

impl<T: Mul + Mul<Output = T> + Copy> Mul<T> for Vec2<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self {
        Self {
            inner: [self.inner[0] * rhs, self.inner[1] * rhs],
        }
    }
}

impl<T: Div + Div<Output = T> + Copy> Div<T> for Vec2<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self {
        Self {
            inner: [self.inner[0] / rhs, self.inner[1] / rhs],
        }
    }
}

impl<T: Eq> Eq for Vec2<T> {}

impl<T> From<[T; 2]> for Vec2<T> {
    fn from(inner: [T; 2]) -> Self {
        Self { inner }
    }
}

impl<T> From<(T, T)> for Vec2<T> {
    fn from(inner: (T, T)) -> Self {
        Self {
            inner: [inner.0, inner.1],
        }
    }
}

impl<T> Deref for Vec2<T> {
    type Target = [T; 2];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Vec2<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
