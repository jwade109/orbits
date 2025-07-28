use crate::input::InputState;
use crate::scenes::CameraProjection;
use bevy::input::keyboard::KeyCode;
use starling::math::Vec2;
use starling::prelude::PHYSICS_CONSTANT_DELTA_TIME;

#[derive(Debug, Clone, Copy)]
pub struct LinearCameraController {
    center: Vec2,
    target_center: Vec2,
    scale: f32,
    target_scale: f32,
    speed: f32,
}

impl CameraProjection for LinearCameraController {
    fn origin(&self) -> Vec2 {
        self.center
    }

    fn scale(&self) -> f32 {
        self.scale()
    }
}

impl LinearCameraController {
    pub fn new(center: Vec2, scale: f32, speed: f32) -> Self {
        let scale = scale.log2();

        Self {
            center,
            target_center: center,
            scale,
            target_scale: scale,
            speed,
        }
    }

    pub fn scale(&self) -> f32 {
        2.0f32.powf(self.scale)
    }

    pub fn on_game_tick(&mut self) {
        const SCALE_SMOOTHING: f32 = 0.1;
        const CENTER_SMOOTHING: f32 = 0.1;

        let dt = PHYSICS_CONSTANT_DELTA_TIME.to_secs();
        self.scale += (self.target_scale - self.scale) * ((dt / SCALE_SMOOTHING).exp() - 1.0);
        self.center += (self.target_center - self.center) * ((dt / CENTER_SMOOTHING).exp() - 1.0)
    }

    pub fn handle_input(&mut self, input: &InputState) {
        const SCROLL_WHEEL_DELTA: f32 = 0.5;
        const BUTTON_ZOOM_SPEED: f32 = 0.05;

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
            self.target_center = Vec2::ZERO;
            self.target_scale = 1.0;
        }
    }
}
