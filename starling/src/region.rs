use crate::aabb::AABB;
use glam::f32::Vec2;

#[derive(Debug, Copy, Clone)]
pub enum Region {
    AABB(AABB),
    OrbitRange(f32, f32),
}

impl Region {
    pub fn aabb(a: Vec2, b: Vec2) -> Self {
        Region::AABB(AABB::from_arbitrary(a, b))
    }

    pub fn range(a: Vec2, b: Vec2) -> Self {
        let r1 = a.length();
        let r2 = b.length();
        Region::OrbitRange(r1.min(r2), r1.max(r2))
    }

    pub fn contains(&self, p: Vec2) -> bool {
        match self {
            Region::AABB(aabb) => aabb.contains(p),
            Region::OrbitRange(a, b) => {
                let r = p.length();
                *a <= r && r <= *b
            }
        }
    }
}
