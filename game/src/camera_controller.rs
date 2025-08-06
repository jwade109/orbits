use crate::input::InputState;
use crate::scenes::CameraProjection;
use bevy::input::keyboard::KeyCode;
use starling::math::DVec2;
use starling::prelude::PHYSICS_CONSTANT_DELTA_TIME;

#[derive(Debug, Clone, Copy)]
pub struct LinearCameraController {
    center: DVec2,
    target_center: DVec2,
    scale: f64,
    target_scale: f64,
    speed: f64,
}

impl CameraProjection for LinearCameraController {
    fn origin(&self) -> DVec2 {
        self.center
    }

    fn scale(&self) -> f64 {
        self.scale()
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
        }
    }

    pub fn scale(&self) -> f64 {
        2.0f64.powf(self.scale)
    }

    pub fn on_game_tick(&mut self) {
        const SCALE_SMOOTHING: f64 = 0.1;
        const CENTER_SMOOTHING: f64 = 0.1;

        let dt = PHYSICS_CONSTANT_DELTA_TIME.to_secs_f64();
        self.scale += (self.target_scale - self.scale) * ((dt / SCALE_SMOOTHING).exp() - 1.0);
        self.center += (self.target_center - self.center) * ((dt / CENTER_SMOOTHING).exp() - 1.0)
    }

    pub fn follow(&mut self, p: DVec2) {
        self.center = p;
        self.target_center = p;
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
        if input.is_pressed(KeyCode::KeyR) && input.is_pressed(KeyCode::ShiftLeft) {
            self.target_center = DVec2::ZERO;
            self.target_scale = 1.0;
        }
    }
}
