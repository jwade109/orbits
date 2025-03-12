use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use starling::aabb::AABB;

#[derive(Resource, Debug)]
pub struct CameraState {
    pub center: Vec2,
    pub target_scale: f32,
    pub actual_scale: f32,
    pub mouse_screen_pos: Option<Vec2>,
    pub mouse_down_pos: Option<Vec2>,
    pub window_dims: Vec2,
}

impl CameraState {}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            target_scale: 4.0,
            actual_scale: 4.0,
            mouse_screen_pos: None,
            mouse_down_pos: None,
            window_dims: Vec2::ZERO,
        }
    }
}

impl CameraState {
    pub fn game_bounds(&self) -> AABB {
        AABB::new(self.center, self.window_dims * self.actual_scale)
    }

    pub fn window_bounds(&self) -> AABB {
        AABB::new(self.window_dims / 2.0, self.window_dims)
    }

    pub fn mouse_pos(&self) -> Option<Vec2> {
        let gb = self.game_bounds();
        let wb = self.window_bounds();
        Some(wb.map(gb, self.mouse_screen_pos?))
    }

    pub fn mouse_down_pos(&self) -> Option<Vec2> {
        let p = self.mouse_down_pos?;
        let gb = self.game_bounds();
        let wb = self.window_bounds();
        Some(wb.map(gb, p))
    }

    pub fn on_scroll(&mut self, scroll: &[&MouseWheel]) {
        for ev in scroll {
            if ev.y > 0.0 {
                self.target_scale /= 1.3;
            } else {
                self.target_scale *= 1.3;
            }
        }
    }

    pub fn on_mouse_click(&mut self, buttons: &ButtonInput<MouseButton>) {
        if buttons.just_pressed(MouseButton::Left) {
            self.mouse_down_pos = self.mouse_screen_pos;
        }
        if buttons.just_released(MouseButton::Left) {
            self.mouse_down_pos = None;
        }
    }

    pub fn on_mouse_move(&mut self, windows: Query<&Window, With<PrimaryWindow>>) {
        let (w, p) = match windows.get_single() {
            Ok(w) => (w, w.cursor_position()),
            Err(_) => {
                self.mouse_screen_pos = None;
                return;
            }
        };
        self.mouse_screen_pos = p.map(|p| Vec2::new(p.x, w.height() - p.y));
        self.window_dims = Vec2::new(w.width(), w.height());
    }
}
