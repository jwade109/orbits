use crate::aabb::AABB;
use crate::math::*;
use crate::orbits::SparseOrbit;

#[derive(Debug, Copy, Clone)]
pub enum Region {
    AABB(AABB),
    OrbitRange(SparseOrbit, SparseOrbit),
    NearOrbit(SparseOrbit, f64),
}

#[deprecated(note = "Contains f32")]
impl Region {
    pub fn aabb(a: DVec2, b: DVec2) -> Self {
        Region::AABB(AABB::from_arbitrary(a.as_vec2(), b.as_vec2()))
    }

    pub fn orbit(a: SparseOrbit, b: SparseOrbit) -> Self {
        Region::OrbitRange(a, b)
    }

    pub fn near_orbit(orbit: SparseOrbit, dist: f64) -> Self {
        Region::NearOrbit(orbit, dist)
    }

    pub fn contains(&self, p: DVec2) -> bool {
        match self {
            Region::AABB(aabb) => aabb.contains(p.as_vec2()),
            Region::NearOrbit(orbit, dist) => {
                orbit.nearest_along_track(p).0.pos.distance(p) < *dist
            }
            _ => false,
        }
    }
}
