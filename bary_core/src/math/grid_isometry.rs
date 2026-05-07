use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone, Copy, Deserialize, Serialize)]
pub struct GridIsometry2d {
    pub translation: IVec2,
    pub rotation: Rotation,
}

impl GridIsometry2d {
    pub const ZERO: Self = Self {
        translation: IVec2::ZERO,
        rotation: Rotation::East,
    };

    pub fn new(translation: impl Into<IVec2>, rotation: Rotation) -> Self {
        Self {
            translation: translation.into(),
            rotation,
        }
    }

    pub fn from_pos(pos: IVec2) -> Self {
        Self {
            translation: pos,
            rotation: Rotation::East,
        }
    }

    pub fn local_x(&self) -> IVec2 {
        match self.rotation {
            Rotation::East => IVec2::X,
            Rotation::North => IVec2::Y,
            Rotation::West => -IVec2::X,
            Rotation::South => -IVec2::Y,
        }
    }

    pub fn local_y(&self) -> IVec2 {
        rotate_90(self.local_x())
    }

    pub fn offset(&self, offset: IVec2) -> Self {
        let mut ret = *self;
        ret.translation += ret.local_x() * offset.x + ret.local_y() * offset.y;
        ret
    }

    pub fn with_rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }
}

impl std::ops::Mul for GridIsometry2d {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        let mut ret = self.offset(rhs.translation);
        ret.rotation = ret.rotation + rhs.rotation;
        ret
    }
}

impl std::ops::Add for GridIsometry2d {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.translation + rhs.translation,
            self.rotation + rhs.rotation,
        )
    }
}

impl From<(i32, i32, Rotation)> for GridIsometry2d {
    fn from((x, y, r): (i32, i32, Rotation)) -> Self {
        Self {
            translation: IVec2::new(x, y),
            rotation: r,
        }
    }
}

impl From<(IVec2, Rotation)> for GridIsometry2d {
    fn from((translation, rotation): (IVec2, Rotation)) -> Self {
        Self {
            translation,
            rotation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_isometry_mul() {
        let a = GridIsometry2d::new((2, 4), Rotation::East);
        let b = GridIsometry2d::new((1, -3), Rotation::North);
        let c = GridIsometry2d::new((-1, 12), Rotation::North);

        let iso: GridIsometry2d = a * b * c;

        assert_eq!(iso.translation, (-9, 0).into());
        assert_eq!(iso.rotation, Rotation::West);
    }
}
