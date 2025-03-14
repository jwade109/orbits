use bevy::prelude::*;

use starling::aabb::AABB;

#[derive(Resource, Debug)]
pub struct CameraState {
    pub world_center: Vec2,
    pub actual_scale: f32,
    pub window_dims: Vec2,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            world_center: Vec2::ZERO,
            actual_scale: 4.0,
            window_dims: Vec2::ZERO,
        }
    }
}

impl CameraState {
    pub fn world_bounds(&self) -> AABB {
        AABB::new(self.world_center, self.window_dims * self.actual_scale)
    }

    pub fn viewport_bounds(&self) -> AABB {
        AABB::new(self.window_dims / 2.0, self.window_dims)
    }
}
