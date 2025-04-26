use crate::mouse::InputState;
use bevy::input::keyboard::KeyCode;
use starling::prelude::PI;

#[derive(Debug, Clone, Copy)]
pub struct TelescopeContext {
    pub azimuth: f32,
    pub elevation: f32,
    pub angular_radius: f32,
    pub target_az: f32,
    pub target_el: f32,
}

impl TelescopeContext {
    pub fn new() -> Self {
        TelescopeContext {
            azimuth: 0.0,
            elevation: 0.0,
            target_az: 0.0,
            target_el: 0.0,
            angular_radius: 1.0,
        }
    }

    pub fn step(&mut self, input: &InputState) {
        if input.is_pressed(KeyCode::Equal) {
            self.angular_radius *= 0.96;
        }
        if input.is_pressed(KeyCode::Minus) {
            self.angular_radius /= 0.96;
        }
        if input.is_pressed(KeyCode::KeyD) {
            self.target_az += 0.01 * self.angular_radius;
        }
        if input.is_pressed(KeyCode::KeyA) {
            self.target_az -= 0.01 * self.angular_radius;
        }
        if input.is_pressed(KeyCode::KeyW) {
            self.target_el += 0.01 * self.angular_radius;
        }
        if input.is_pressed(KeyCode::KeyS) {
            self.target_el -= 0.01 * self.angular_radius;
        }

        self.angular_radius = self.angular_radius.clamp(0.05, PI / 2.0);

        self.azimuth += (self.target_az - self.azimuth) * 0.1;
        self.elevation += (self.target_el - self.elevation) * 0.1;
    }
}
