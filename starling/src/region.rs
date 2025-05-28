use crate::aabb::AABB;
use crate::orbits::SparseOrbit;
use glam::f32::Vec2;

#[derive(Debug, Copy, Clone)]
pub enum Region {
    AABB(AABB),
    OrbitRange(SparseOrbit, SparseOrbit),
    NearOrbit(SparseOrbit, f32),
}

impl Region {
    pub fn aabb(a: Vec2, b: Vec2) -> Self {
        Region::AABB(AABB::from_arbitrary(a, b))
    }

    pub fn orbit(a: SparseOrbit, b: SparseOrbit) -> Self {
        Region::OrbitRange(a, b)
    }

    pub fn near_orbit(orbit: SparseOrbit, dist: f32) -> Self {
        Region::NearOrbit(orbit, dist)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        match self {
            Region::AABB(aabb) => aabb.contains(p),
            Region::NearOrbit(orbit, dist) => {
                orbit.nearest_along_track(p).0.pos_f32().distance(p) < *dist
            }
            _ => false,
        }
    }
}
