use crate::input::InputState;
use bevy::input::keyboard::KeyCode;
use starling::math::DVec2;
use starling::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct LinearCameraController {
    center: DVec2,
    target_center: DVec2,
    scale: f64,
    target_scale: f64,
    speed: f64,
    parent: EntityId,
    offset: DVec2,
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

    fn parent(&self) -> EntityId {
        self.parent
    }
}

impl std::fmt::Display for LinearCameraController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{:0.1}", self.parent, self.offset)
    }
}

impl LinearCameraController {
    pub fn new(center: DVec2, scale: f64, speed: f64) -> Self {
        let scale = scale.log2();

        Self {
            center,
            target_center: center,
            scale,
            target_scale: scale,
            speed,
            parent: EntityId(0),
            offset: DVec2::ZERO,
        }
    }

    pub fn scale(&self) -> f64 {
        2.0f64.powf(self.scale)
    }

    pub fn clear_offset(&mut self) {
        self.target_center = DVec2::ZERO;
    }

    pub fn on_game_tick(&mut self) {
        const SCALE_SMOOTHING: f64 = 0.1;
        const CENTER_SMOOTHING: f64 = 0.1;

        let dt = PHYSICS_CONSTANT_DELTA_TIME.to_secs_f64();
        self.scale += (self.target_scale - self.scale) * ((dt / SCALE_SMOOTHING).exp() - 1.0);
        self.offset += (self.target_center - self.offset) * ((dt / CENTER_SMOOTHING).exp() - 1.0)
    }

    pub fn follow(&mut self, parent: EntityId, p: DVec2) {
        if parent != self.parent {
            self.target_center = DVec2::ZERO;
            self.offset = self.center + self.offset - p;
        }
        self.parent = parent;
        self.center = p;
    }

    pub fn offset(&self) -> DVec2 {
        self.offset
    }

    pub fn parent(&self) -> EntityId {
        self.parent
    }

    pub fn handle_input(&mut self, input: &InputState) {
        const SCROLL_WHEEL_DELTA: f64 = 0.5;
        const BUTTON_ZOOM_SPEED: f64 = 0.05;

        let speed = self.speed * 0.01;

        if input.is_scroll_down() {
            self.target_scale -= SCROLL_WHEEL_DELTA;
        }
        if input.is_scroll_up() {
            self.target_scale += SCROLL_WHEEL_DELTA;
        }

        if input.is_pressed(KeyCode::Equal) {
            self.target_scale += BUTTON_ZOOM_SPEED;
        }
        if input.is_pressed(KeyCode::Minus) {
            self.target_scale -= BUTTON_ZOOM_SPEED;
        }

        if input.is_pressed(KeyCode::KeyD) {
            self.target_center.x += speed / self.scale();
        }
        if input.is_pressed(KeyCode::KeyA) {
            self.target_center.x -= speed / self.scale();
        }
        if input.is_pressed(KeyCode::KeyW) {
            self.target_center.y += speed / self.scale();
        }
        if input.is_pressed(KeyCode::KeyS) {
            self.target_center.y -= speed / self.scale();
        }

        self.target_scale = self.target_scale.clamp(-22.0, 10.0);
    }
}

pub trait CameraProjection {
    /// World to camera transform
    fn w2c(&self, p: DVec2) -> Vec2 {
        graphics_cast((p - self.origin()) * self.scale())
    }

    fn w2c_aabb(&self, aabb: AABB) -> AABB {
        let a = aabb.lower().as_dvec2();
        let b = aabb.upper().as_dvec2();
        AABB::from_arbitrary(self.w2c(a), self.w2c(b))
    }

    /// Camera to world transform
    fn c2w(&self, p: Vec2) -> DVec2 {
        p.as_dvec2() / self.scale() + self.origin()
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

    fn parent(&self) -> EntityId;
}
