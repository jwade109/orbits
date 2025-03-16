use crate::aabb::AABB;
use crate::orbits::SparseOrbit;
use glam::f32::Vec2;

#[derive(Debug, Copy, Clone)]
pub enum Region {
    AABB(AABB),
    AltitudeRange(f32, f32),
    OrbitRange(SparseOrbit, SparseOrbit),
    NearOrbit(SparseOrbit, f32),
}

impl Region {
    pub fn aabb(a: Vec2, b: Vec2) -> Self {
        Region::AABB(AABB::from_arbitrary(a, b))
    }

    pub fn altitude(a: Vec2, b: Vec2) -> Self {
        let r1 = a.length();
        let r2 = b.length();
        Region::AltitudeRange(r1.min(r2), r1.max(r2))
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
            Region::AltitudeRange(a, b) => {
                let r = p.length();
                *a <= r && r <= *b
            }
            Region::NearOrbit(orbit, dist) => {
                orbit.nearest_along_track(p).0.pos.distance(p) < *dist
            }
            _ => false,
        }
    }
}
