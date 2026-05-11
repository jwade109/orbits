use crate::input::InputState;
use crate::settings::Settings;
use bary_core::prelude::*;
use bevy::input::keyboard::KeyCode;
use bevy::math::DVec2;

#[derive(Debug, Clone, Copy)]
pub struct LinearCameraController {
    center: DVec2,
    target_offset: DVec2,
    offset: DVec2,

    view_distance: f64,
    target_view_distance: f64,

    angle: f64,
    target_angle: f64,
}

impl CameraProjection for LinearCameraController {
    fn origin(&self) -> DVec2 {
        self.center + self.offset
    }

    fn scale(&self) -> f64 {
        self.scale()
    }

    fn offset(&self) -> DVec2 {
        self.offset
    }

    fn distance(&self) -> f64 {
        self.view_distance
    }

    fn angle(&self) -> f64 {
        self.angle
    }
}

impl LinearCameraController {
    pub fn new(center: DVec2, distance: f64) -> Self {
        Self {
            center,
            target_offset: DVec2::ZERO,
            offset: DVec2::ZERO,
            view_distance: distance,
            target_view_distance: distance,
            angle: 0.0,
            target_angle: 0.0,
        }
    }

    pub fn scale(&self) -> f64 {
        1.0 / self.view_distance
    }

    pub fn clear_offset(&mut self) {
        self.target_offset = DVec2::ZERO;
    }

    pub fn on_game_tick(&mut self) {
        const CENTER_SMOOTHING: f64 = 0.1;
        const SCALE_SMOOTHING: f64 = 0.1;
        const ANGLE_SMOOTHING: f64 = 0.1;

        let dt = PHYSICS_CONSTANT_DELTA_TIME.to_secs_f64();
        self.offset += (self.target_offset - self.offset) * ((dt / CENTER_SMOOTHING).exp() - 1.0);
        self.view_distance +=
            (self.target_view_distance - self.view_distance) * ((dt / SCALE_SMOOTHING).exp() - 1.0);
        self.angle += (self.target_angle - self.angle) * ((dt / ANGLE_SMOOTHING).exp() - 1.0);
    }

    pub fn set_center(&mut self, pos: DVec2) {
        self.center = pos;
    }

    pub fn set_target_offset(&mut self, offset: DVec2) {
        self.target_offset = offset;
    }

    pub fn set_target_view_distance(&mut self, dist: f64) {
        self.target_view_distance = dist;
    }

    pub fn offset(&self) -> DVec2 {
        self.offset
    }

    pub fn handle_input(&mut self, input: &InputState, settings: &Settings) {
        let speed = 9.0 * settings.camera_pan_sensitivity;

        if input.is_scroll_down() {
            self.target_view_distance *= 1.4 * settings.camera_scroll_sensitivity;
        }
        if input.is_scroll_up() {
            self.target_view_distance /= 1.4 * settings.camera_scroll_sensitivity;
        }

        if input.is_pressed(KeyCode::Equal) {
            // self.target_scale += BUTTON_ZOOM_SPEED;
            self.target_view_distance /= 1.02 * settings.camera_zoom_button_sensitivity;
        }
        if input.is_pressed(KeyCode::Minus) {
            // self.target_scale -= BUTTON_ZOOM_SPEED;
            self.target_view_distance *= 1.02 * settings.camera_zoom_button_sensitivity;
        }

        let mut delta = DVec2::ZERO;

        if input.is_pressed(KeyCode::KeyD) {
            delta.x += speed / self.scale();
        }
        if input.is_pressed(KeyCode::KeyA) {
            delta.x -= speed / self.scale();
        }
        if input.is_pressed(KeyCode::KeyW) {
            delta.y += speed / self.scale();
        }
        if input.is_pressed(KeyCode::KeyS) {
            delta.y -= speed / self.scale();
        }

        self.target_offset += rotate_f64(delta, self.angle);

        self.target_view_distance = self.target_view_distance.clamp(0.001, 600000.0);
    }
}

pub trait CameraProjection {
    /// World to camera transform
    fn w2c(&self, p: DVec2) -> Vec2 {
        let delta = p - self.origin();
        let delta = rotate_f64(delta, -self.angle());
        graphics_cast(delta * self.scale())
    }

    fn w2c_aabb(&self, aabb: AABB) -> AABB {
        let a = aabb.lower().as_dvec2();
        let b = aabb.upper().as_dvec2();
        AABB::from_arbitrary(self.w2c(a), self.w2c(b))
    }

    /// Camera to world transform
    fn c2w(&self, p: Vec2) -> DVec2 {
        let delta = p.as_dvec2() / self.scale();
        let delta = rotate_f64(delta, self.angle());
        delta + self.origin()
    }

    #[allow(unused)]
    fn c2w_aabb(&self, aabb: AABB) -> AABB {
        let a = aabb.lower();
        let b = aabb.upper();
        AABB::from_arbitrary(
            aabb_stopgap_cast(self.c2w(a)),
            aabb_stopgap_cast(self.c2w(b)),
        )
    }

    fn origin(&self) -> DVec2;

    fn scale(&self) -> f64;

    fn offset(&self) -> DVec2;

    fn distance(&self) -> f64;

    fn angle(&self) -> f64;
}

pub fn camera_span_meters(screen_bounds: Vec2, ctx: &impl CameraProjection) -> DVec2 {
    screen_bounds.as_dvec2() / ctx.scale()
}
