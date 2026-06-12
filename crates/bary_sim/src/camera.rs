use bary_core::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub isometry: Isometry2d,
    /// camera zoom in raylib units, or whatever
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            isometry: Isometry2d::ZERO,
            zoom: 1.0,
        }
    }
}
