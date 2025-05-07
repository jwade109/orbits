use crate::mouse::InputState;
use crate::scenes::CameraProjection;
use bevy::input::keyboard::KeyCode;
use starling::prelude::*;

#[derive(Debug)]
pub struct RPOContext {
    center: Vec2,
    target_center: Vec2,
    scale: f32,
    target_scale: f32,
}

impl CameraProjection for RPOContext {
    fn origin(&self) -> Vec2 {
        self.center
    }

    fn scale(&self) -> f32 {
        self.scale
    }
}

impl RPOContext {
    pub fn new() -> Self {
        Self {
            center: Vec2::ZERO,
            target_center: Vec2::ZERO,
            scale: 1.0,
            target_scale: 1.0,
        }
    }

    pub fn step(&mut self, input: &InputState, dt: f32) {
        let speed = 16.0 * dt * 100.0;

        if input.is_scroll_down() {
            self.target_scale /= 1.5;
        }
        if input.is_scroll_up() {
            self.target_scale *= 1.5;
        }

        if input.is_pressed(KeyCode::Equal) {
            self.target_scale *= 1.03;
        }
        if input.is_pressed(KeyCode::Minus) {
            self.target_scale /= 1.03;
        }
        if input.is_pressed(KeyCode::KeyD) {
            self.target_center.x += speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyA) {
            self.target_center.x -= speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyW) {
            self.target_center.y += speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyS) {
            self.target_center.y -= speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyR) {
            self.target_center = Vec2::ZERO;
            self.target_scale = 1.0;
        }

        self.scale += (self.target_scale - self.scale) * 0.1;
        self.center += (self.target_center - self.center) * 0.1;
    }
}
