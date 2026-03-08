use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone, Copy, Deserialize, Serialize)]
pub struct Isometry2d {
    pub translation: Vec2,
    pub rotation: f32,
}

impl Isometry2d {
    pub const ZERO: Self = Self {
        translation: Vec2::ZERO,
        rotation: 0.0,
    };

    pub fn new(translation: Vec2, rotation: f32) -> Self {
        Self {
            translation,
            rotation,
        }
    }

    pub fn from_pos(pos: Vec2) -> Self {
        Self {
            translation: pos,
            rotation: 0.0,
        }
    }

    pub fn local_x(&self) -> Vec2 {
        rotate(Vec2::X, self.rotation)
    }

    pub fn local_y(&self) -> Vec2 {
        rotate(Vec2::Y, self.rotation)
    }

    pub fn offset(&self, offset: Vec2) -> Self {
        let mut ret = *self;
        ret.translation += ret.local_x() * offset.x + ret.local_y() * offset.y;
        ret
    }

    pub fn to_tuple(&self) -> (f32, f32, f32) {
        (self.translation.x, self.translation.y, self.rotation)
    }
}

impl std::ops::Mul for Isometry2d {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        let mut ret = self.offset(rhs.translation);
        ret.rotation += rhs.rotation;
        ret
    }
}

impl std::ops::Add for Isometry2d {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.translation + rhs.translation,
            self.rotation + rhs.rotation,
        )
    }
}

impl From<(f32, f32, f32)> for Isometry2d {
    fn from((x, y, r): (f32, f32, f32)) -> Self {
        Self {
            translation: Vec2::new(x, y),
            rotation: r,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isometry_mul() {
        let a = Isometry2d::new((2.3, 4.0).into(), 0.1);
        let b = Isometry2d::new((0.5, -3.4).into(), 0.6);
        let c = Isometry2d::new((-0.4, 12.1).into(), -0.9);

        let iso = a * b * c;

        assert_eq!(iso.translation, (-4.9640365, 9.663805).into());
        assert_eq!(iso.rotation, -0.19999993);
    }
}
