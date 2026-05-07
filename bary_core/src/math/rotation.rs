use super::math::{IVec2, PI_64};
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    Sequence,
    Serialize,
    Deserialize,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
pub enum Rotation {
    #[default]
    East,
    North,
    West,
    South,
}

impl Rotation {
    pub fn to_angle(&self) -> f64 {
        match self {
            Self::East => 0.0,
            Self::North => PI_64 * 0.5,
            Self::West => PI_64,
            Self::South => PI_64 * 1.5,
        }
    }

    pub fn to_dir(&self) -> IVec2 {
        match self {
            Self::East => IVec2::X,
            Self::North => IVec2::Y,
            Self::West => -IVec2::X,
            Self::South => -IVec2::Y,
        }
    }

    pub fn next(&self) -> Self {
        enum_iterator::next_cycle(self)
    }

    pub fn prev(&self) -> Self {
        enum_iterator::previous_cycle(self)
    }

    pub fn all() -> impl Iterator<Item = Self> {
        enum_iterator::all::<Self>()
    }
}

impl std::ops::Add for Rotation {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (x, Self::East) => x,
            (Self::East, x) => x,
            (Self::North, Self::North) => Self::West,
            (Self::North, Self::West) => Self::South,
            (Self::North, Self::South) => Self::East,
            (Self::West, Self::North) => Self::South,
            (Self::West, Self::West) => Self::East,
            (Self::West, Self::South) => Self::North,
            (Self::South, Self::North) => Self::East,
            (Self::South, Self::West) => Self::North,
            (Self::South, Self::South) => Self::West,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_addition() {
        for r1 in enum_iterator::all::<Rotation>() {
            for r2 in enum_iterator::all::<Rotation>() {
                let r = r1 + r2;

                let a = r1 as u8;
                let b = r2 as u8;
                let c = r as u8;

                let sum = (a + b) % 4;

                assert_eq!(sum, c);
            }
        }
    }
}
