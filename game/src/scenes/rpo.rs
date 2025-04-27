use crate::mouse::InputState;
use bevy::input::keyboard::KeyCode;
use starling::prelude::*;

#[derive(Debug)]
pub struct RPOContext {
    pub center: Vec2,
    pub target_center: Vec2,
    pub zoom: f32,
}

impl RPOContext {
    pub fn new() -> Self {
        Self {
            center: Vec2::ZERO,
            target_center: Vec2::ZERO,
            zoom: 1.0,
        }
    }

    pub fn step(&mut self, input: &InputState) {
        let speed = 16.0;
        if input.is_pressed(KeyCode::Equal) {
            self.zoom *= 1.03;
        }
        if input.is_pressed(KeyCode::Minus) {
            self.zoom /= 1.03;
        }
        if input.is_pressed(KeyCode::KeyD) {
            self.target_center.x += speed / self.zoom;
        }
        if input.is_pressed(KeyCode::KeyA) {
            self.target_center.x -= speed / self.zoom;
        }
        if input.is_pressed(KeyCode::KeyW) {
            self.target_center.y += speed / self.zoom;
        }
        if input.is_pressed(KeyCode::KeyS) {
            self.target_center.y -= speed / self.zoom;
        }
        if input.is_pressed(KeyCode::KeyR) {
            self.target_center = Vec2::ZERO;
            self.zoom = 1.0;
        }

        self.center += (self.target_center - self.center) * 0.1;
    }
}
