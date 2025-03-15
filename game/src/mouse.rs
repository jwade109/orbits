use crate::ui::InteractionEvent;
use bevy::prelude::*;
use core::time::Duration;
use starling::prelude::AABB;

const DOUBLE_CLICK_DURATION: Duration = Duration::from_millis(200);

#[derive(Component, Debug, Default)]
pub struct MouseState {
    last_click: Option<Duration>,
    position: Option<Vec2>,
    left_click: Option<Vec2>,
    right_click: Option<Vec2>,
    middle_click: Option<Vec2>,

    viewport_bounds: AABB,
    world_bounds: AABB,
    scale: f32,
}

impl MouseState {
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn current(&self) -> Option<Vec2> {
        self.position
    }

    pub fn left(&self) -> Option<Vec2> {
        self.left_click
    }

    pub fn right(&self) -> Option<Vec2> {
        self.right_click
    }

    pub fn middle(&self) -> Option<Vec2> {
        self.middle_click
    }

    fn viewport_to_world(&self, p: Vec2) -> Vec2 {
        self.viewport_bounds.map(self.world_bounds, p)
    }

    pub fn current_world(&self) -> Option<Vec2> {
        Some(self.viewport_to_world(self.current()?))
    }

    pub fn left_world(&self) -> Option<Vec2> {
        Some(self.viewport_to_world(self.left()?))
    }

    pub fn right_world(&self) -> Option<Vec2> {
        Some(self.viewport_to_world(self.right()?))
    }

    pub fn middle_world(&self) -> Option<Vec2> {
        Some(self.viewport_to_world(self.middle()?))
    }
}

pub fn update_mouse_state(
    win: Single<&Window>,
    buttons: Res<ButtonInput<MouseButton>>,
    camera: Single<&Transform, With<Camera2d>>,
    mut state: Single<&mut MouseState>,
    mut events: EventWriter<InteractionEvent>,
    time: Res<Time>,
) {
    let dims = Vec2::new(win.width(), win.height());

    state.viewport_bounds = AABB::new(dims / 2.0, dims);
    state.world_bounds = AABB::new(camera.translation.xy(), dims * camera.scale.z);
    state.scale = camera.scale.z;

    if let Some(p) = win.cursor_position() {
        let p = Vec2::new(p.x, dims.y - p.y);
        if state.position != Some(p) {
            state.position = Some(p);
        }
    } else {
        if state.position.is_some() {
            state.position = None;
        }
    }

    if buttons.just_pressed(MouseButton::Left) {
        state.left_click = state.position;
        let now = time.elapsed();
        if let Some(prev) = state.last_click {
            let dt = now - prev;
            if let Some(p) = (dt < DOUBLE_CLICK_DURATION)
                .then(|| state.left_click)
                .flatten()
            {
                events.send(InteractionEvent::DoubleClick(p));
            }
        }
        state.last_click = Some(now);
    } else if buttons.just_released(MouseButton::Left) {
        state.left_click = None;
    }

    if buttons.just_pressed(MouseButton::Right) {
        state.right_click = state.position;
    } else if buttons.just_released(MouseButton::Right) {
        state.right_click = None;
    }

    if buttons.just_pressed(MouseButton::Middle) {
        state.middle_click = state.position;
    } else if buttons.just_released(MouseButton::Middle) {
        state.middle_click = None;
    }
}
