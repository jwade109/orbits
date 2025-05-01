use crate::mouse::InputState;
use crate::scenes::CameraProjection;
use bevy::input::keyboard::KeyCode;
use starling::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct TelescopeContext {
    azimuth: f32,
    elevation: f32,
    angular_radius: f32,
    target_az: f32,
    target_el: f32,
    target_angular_radius: f32,
}

impl CameraProjection for TelescopeContext {
    fn origin(&self) -> Vec2 {
        Vec2::new(self.azimuth, self.elevation)
    }

    fn scale(&self) -> f32 {
        1.0 / self.angular_radius
    }
}

impl TelescopeContext {
    pub fn new() -> Self {
        TelescopeContext {
            azimuth: 0.0,
            elevation: 0.0,
            target_az: 0.0,
            target_el: 0.0,
            angular_radius: 1.1,
            target_angular_radius: 1.0,
        }
    }

    pub fn step(&mut self, input: &InputState) {
        if input.is_scroll_down() {
            self.target_angular_radius *= 1.5;
        }
        if input.is_scroll_up() {
            self.target_angular_radius /= 1.5;
        }

        if input.is_pressed(KeyCode::Equal) {
            self.target_angular_radius *= 0.96;
        }
        if input.is_pressed(KeyCode::Minus) {
            self.target_angular_radius /= 0.96;
        }

        let angular_speed = 0.004;

        if input.is_pressed(KeyCode::KeyD) {
            self.target_az += angular_speed * self.angular_radius;
        }
        if input.is_pressed(KeyCode::KeyA) {
            self.target_az -= angular_speed * self.angular_radius;
        }
        if input.is_pressed(KeyCode::KeyW) {
            self.target_el += angular_speed * self.angular_radius;
        }
        if input.is_pressed(KeyCode::KeyS) {
            self.target_el -= angular_speed * self.angular_radius;
        }
        if input.is_pressed(KeyCode::KeyR) {
            self.target_el = 0.0;
            self.target_az = 0.0;
            self.target_angular_radius = 1.0;
        }

        self.target_angular_radius = self.target_angular_radius.clamp(0.05, PI / 2.0);

        self.angular_radius += (self.target_angular_radius - self.angular_radius) * 0.03;
        self.azimuth += (self.target_az - self.azimuth) * 0.03;
        self.elevation += (self.target_el - self.elevation) * 0.03;
    }
}
