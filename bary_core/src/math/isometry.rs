use crate::math::*;

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct Isometry2d {
    pub translation: Vec2,
    pub rotation: f32,
}

impl Isometry2d {
    pub fn new(translation: Vec2, rotation: f32) -> Self {
        Self {
            translation,
            rotation,
        }
    }

    pub fn local_x(&self) -> Vec2 {
        rotate(Vec2::X, self.rotation)
    }

    pub fn local_y(&self) -> Vec2 {
        rotate(Vec2::Y, self.rotation)
    }
}

impl std::ops::Mul for Isometry2d {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Isometry2d::new(self.translation + rhs.translation, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{Isometry2, Vector2};

    #[test]
    fn isometry_mul() {
        let a = Isometry2d::new((2.3, 4.0).into(), 0.1);
        let b = Isometry2d::new((0.5, -3.4).into(), 0.6);
        let c = Isometry2d::new((-0.4, 12.1).into(), -0.9);

        let iso = a * b * c;

        // assert_eq!(iso.translation, (2.8, 0.5999999).into());
        // assert_eq!(iso.rotation, 0.0);

        let x = Isometry2::new(Vector2::new(2.3, 4.0), 0.1);
        let y = Isometry2::new(Vector2::new(0.5, -3.4), 0.6);
        let z = Isometry2::new(Vector2::new(-0.4, 12.1), -0.9);

        let res = x * y * z;

        assert_eq!(
            res.translation.vector,
            Vector2::new(-4.96403519125163, 9.663805937625362)
        );
        assert_eq!(res.rotation.angle(), -0.20000000000000007);
    }
}
