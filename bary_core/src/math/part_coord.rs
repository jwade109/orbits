use super::aabb::*;
use super::math::*;
use bevy::prelude::{IVec2, Vec2};
use serde::{Deserialize, Serialize};

// TODO reduce scope of this constant
pub const GRID_CELLS_PER_METER: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct PartCoord(pub IVec2);

impl PartCoord {
    pub const CELL_WIDTH: f32 = 1.0 / GRID_CELLS_PER_METER;

    pub const ONE: Self = Self(IVec2::ONE);

    pub fn new(p: impl Into<IVec2>) -> Self {
        Self(p.into())
    }

    pub fn to_meters(&self) -> Vec2 {
        self.0.as_vec2() / GRID_CELLS_PER_METER
    }

    pub fn to_meters_center(&self) -> Vec2 {
        (self.0.as_vec2() + Vec2::splat(0.5)) / GRID_CELLS_PER_METER
    }

    pub fn from_meters_floored(p: impl Into<DVec2>) -> Self {
        Self(vfloor(p.into().as_vec2() * GRID_CELLS_PER_METER))
    }

    pub fn from_meters_rounded(p: impl Into<DVec2>) -> Self {
        Self(vround(p.into().as_vec2() * GRID_CELLS_PER_METER))
    }

    pub fn from_meters_ceiled(p: impl Into<DVec2>) -> Self {
        Self(vceil(p.into().as_vec2() * GRID_CELLS_PER_METER))
    }

    pub fn inner(&self) -> IVec2 {
        self.0
    }

    pub fn to_aabb(&self) -> AABB {
        let lower = self.to_meters();
        let upper = (*self + PartCoord::new(IVec2::ONE)).to_meters();
        AABB::from_arbitrary(lower, upper)
    }

    pub fn in_aabb(aabb: AABB) -> impl Iterator<Item = Self> {
        let lower = Self::from_meters_floored(aabb.lower()).inner();
        let upper = Self::from_meters_floored(aabb.upper()).inner();

        (lower.x..=upper.x)
            .flat_map(move |x| (lower.y..=upper.y).map(move |y| IVec2::new(x, y)))
            .map(|p| Self::new(p))
    }
}

impl std::ops::Add for PartCoord {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for PartCoord {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl std::ops::Sub for PartCoord {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Into<PartCoord> for (i32, i32) {
    fn into(self) -> PartCoord {
        PartCoord(IVec2::new(self.0, self.1))
    }
}

impl Into<PartCoord> for IVec2 {
    fn into(self) -> PartCoord {
        PartCoord(self)
    }
}

impl Into<PartCoord> for UVec2 {
    fn into(self) -> PartCoord {
        PartCoord(self.as_ivec2())
    }
}

impl std::fmt::Display for PartCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0.x, self.0.y)
    }
}
