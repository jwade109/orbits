use crate::camera::Camera;
use bary_core::prelude::*;
use raylib::prelude::*;

pub fn glam_to_raylib(v: Vec2) -> Vector2 {
    Vector2::new(v.x, v.y)
}

pub fn glam_to_raylib_swap_x(v: Vec2) -> Vector2 {
    Vector2::new(-v.x, v.y)
}

pub fn glam_to_raylib_swap_y(v: Vec2) -> Vector2 {
    Vector2::new(v.x, -v.y)
}

pub fn raylib_to_glam(v: Vector2) -> Vec2 {
    Vec2::new(v.x, v.y)
}

pub fn raylib_to_glam_invert_y(v: Vector2) -> Vec2 {
    Vec2::new(v.x, -v.y)
}

pub fn part_isometry(root_isometry: Isometry2d, placement: GridPlacement) -> Isometry2d {
    let part_iso = placement.origin_isometry();

    // TODO replace this with std::ops::Mul
    let rotation = root_isometry.rotation + part_iso.rotation;
    let offset = root_isometry.local_x() * part_iso.translation.x
        + root_isometry.local_y() * part_iso.translation.y;
    Isometry2d::new(root_isometry.translation + offset, rotation)
}

pub fn get_isometry(camera: &Camera) -> Isometry2d {
    Isometry2d {
        translation: camera.target,
        rotation: camera.rotation,
    }
}

pub fn default_camera_2d() -> Camera2D {
    Camera2D {
        offset: Vector2::zero(),
        target: Vector2::zero(),
        rotation: 0.0,
        zoom: 1.0,
    }
}

pub fn screen_to_world(camera: &Camera, screen_pos: Vec2, screen_dims: Vec2) -> Vec2 {
    // this is gross
    let offset = screen_dims / 2.0;
    let delta =
        raylib_to_glam_invert_y(glam_to_raylib(screen_pos) - glam_to_raylib(offset)) / camera.zoom;
    rotate(delta, camera.rotation) + raylib_to_glam(glam_to_raylib(camera.target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_to_world_conversion() {
        let screen_dims = Vec2::new(1080.0, 720.0);

        let center = screen_dims / 2.0;

        let target = Vec2::new(100.0, 120.0);

        let camera = Camera {
            target,
            rotation: 34.0f32.to_radians(),
            zoom: 1.2,
        };

        let screen_pos = center;
        let world_pos = screen_to_world(&camera, screen_pos, screen_dims);
        assert_eq!(world_pos, target);

        let screen_pos = Vec2::new(560.0, 293.0);
        let world_pos = screen_to_world(&camera, screen_pos, screen_dims);
        assert_eq!(world_pos, Vec2::new(82.59569, 175.60782));
    }
}
