use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use starling::aabb::AABB;

#[derive(Debug, PartialEq, Eq)]
pub enum CameraTracking {
    // ExternalTrack,
    TrackingCursor,
    Freewheeling,
}

#[derive(Resource, Debug)]
pub struct CameraState {
    pub cursor: Vec2,
    pub center: Vec2,
    pub easing_lpf: f32,
    pub state: CameraTracking,
    pub target_scale: f32,
    pub actual_scale: f32,
    pub mouse_screen_pos: Option<Vec2>,
    pub mouse_down_pos: Option<Vec2>,
    pub window_dims: Vec2,
}

impl CameraState {
    pub fn track(&mut self, pos: Vec2, state: CameraTracking) {
        if self.state != state {
            self.easing_lpf = 0.1;
        }

        self.center += (pos - self.center) * self.easing_lpf;
        self.easing_lpf += (1.0 - self.easing_lpf) * 0.01;
        self.state = state;
    }
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            cursor: Vec2::ZERO,
            center: Vec2::ZERO,
            easing_lpf: 0.1,
            state: CameraTracking::Freewheeling,
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
        Some(AABB::map(wb, gb, self.mouse_screen_pos?))
    }

    pub fn mouse_down_pos(&self) -> Option<Vec2> {
        let p = self.mouse_down_pos?;
        let gb = self.game_bounds();
        let wb = self.window_bounds();
        Some(AABB::map(wb, gb, p))
    }

    pub fn selection_region(&self) -> Option<AABB> {
        Some(AABB::from_arbitrary(
            self.mouse_pos()?,
            self.mouse_down_pos()?,
        ))
    }

    pub fn on_scroll(&mut self, mut scroll: EventReader<MouseWheel>) {
        for ev in scroll.read() {
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
            self.state = CameraTracking::Freewheeling;
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

    pub fn on_keys(&mut self, keys: &ButtonInput<KeyCode>, dt: f32) {
        let cursor_rate = 1400.0 * self.actual_scale;
        if keys.just_pressed(KeyCode::Equal) {
            self.target_scale /= 1.5;
        }
        if keys.just_pressed(KeyCode::Minus) {
            self.target_scale *= 1.5;
        }
        if keys.pressed(KeyCode::KeyW) {
            self.cursor.y += cursor_rate * dt;
        }
        if keys.pressed(KeyCode::KeyA) {
            self.cursor.x -= cursor_rate * dt;
        }
        if keys.pressed(KeyCode::KeyD) {
            self.cursor.x += cursor_rate * dt;
        }
        if keys.pressed(KeyCode::KeyS) {
            self.cursor.y -= cursor_rate * dt;
        }
    }
}

pub fn update_camera_transform(
    mut query: Query<&mut Transform, With<Camera>>,
    cam: &mut CameraState,
) {
    let mut tf = query.single_mut();

    let s = cam.cursor;
    cam.track(s, CameraTracking::TrackingCursor);

    *tf = tf.with_translation(cam.center.extend(0.0));

    let ds = (cam.target_scale - tf.scale) * 0.5;
    tf.scale += ds;
    cam.actual_scale = tf.scale.x;
}
