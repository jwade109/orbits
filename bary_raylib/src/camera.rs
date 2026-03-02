use bary_core::prelude::*;
use raylib::prelude::*;
use serde::{Deserialize, Serialize};

use crate::utils::glam_to_raylib;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Camera {
    pub isometry: Isometry2d,
    /// camera zoom in raylib units, or whatever
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            isometry: Isometry2d::IDENTITY,
            zoom: 1.0,
        }
    }
}

pub fn to_raylib_camera(camera: &Camera, screen_dims: Vec2) -> Camera2D {
    Camera2D {
        offset: glam_to_raylib(screen_dims) / 2.0,
        target: Vector2::new(
            camera.isometry.translation.x,
            -camera.isometry.translation.y,
        ),
        rotation: camera.isometry.rotation.to_degrees(),
        zoom: camera.zoom,
    }
}
