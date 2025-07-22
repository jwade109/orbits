use crate::math::PI;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Sequence, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rotation {
    East,
    North,
    West,
    South,
}

impl Rotation {
    pub fn to_angle(&self) -> f32 {
        match self {
            Self::East => 0.0,
            Self::North => PI * 0.5,
            Self::West => PI,
            Self::South => PI * 1.5,
        }
    }
}
