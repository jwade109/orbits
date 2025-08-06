use crate::math::*;

pub fn graphics_cast(p: DVec2) -> Vec2 {
    p.as_vec2()
}

/// cast for graphics compatibility
pub fn gcast(x: f64) -> f32 {
    x as f32
}

pub fn aabb_stopgap_cast(p: DVec2) -> Vec2 {
    p.as_vec2()
}

/// note to revisit the callsite
pub fn revisit<T>(x: T) -> T {
    x
}
