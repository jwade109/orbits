use bevy::prelude::*;

use starling::aabb::AABB;

#[derive(Debug)]
pub struct OrbitalCameraState {
    pub world_center: Vec2,
    pub actual_scale: f32,
}

impl Default for OrbitalCameraState {
    fn default() -> Self {
        Self {
            world_center: Vec2::ZERO,
            actual_scale: 4.0,
        }
    }
}

impl OrbitalCameraState {
    pub fn world_bounds(&self, window_dims: Vec2) -> AABB {
        AABB::new(self.world_center, window_dims * self.actual_scale)
    }
}
