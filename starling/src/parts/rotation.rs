use crate::math::PI_64;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Sequence, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum Rotation {
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
}
