use crate::ui::InteractionEvent;
use bevy::prelude::*;
use core::time::Duration;

const DOUBLE_CLICK_DURATION: Duration = Duration::from_millis(200);

#[derive(Component, Debug, Default)]
pub struct MouseState {
    last_click: Option<Duration>,
    position: Option<Vec2>,
    left_click: Option<Vec2>,
    right_click: Option<Vec2>,
    middle_click: Option<Vec2>,
}

impl MouseState {
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

    pub fn current_world(&self, cam: (&Camera, &GlobalTransform)) -> Option<Vec2> {
        cam.0.viewport_to_world_2d(cam.1, self.current()?).ok()
    }

    pub fn left_world(&self, cam: (&Camera, &GlobalTransform)) -> Option<Vec2> {
        cam.0.viewport_to_world_2d(cam.1, self.left()?).ok()
    }

    pub fn right_world(&self, cam: (&Camera, &GlobalTransform)) -> Option<Vec2> {
        cam.0.viewport_to_world_2d(cam.1, self.right()?).ok()
    }

    pub fn middle_world(&self, cam: (&Camera, &GlobalTransform)) -> Option<Vec2> {
        cam.0.viewport_to_world_2d(cam.1, self.middle()?).ok()
    }
}

pub fn cursor_position(
    win: Single<&Window>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut state: Single<&mut MouseState>,
    mut events: EventWriter<InteractionEvent>,
    cam: Single<(&Camera, &GlobalTransform)>,
    time: Res<Time>,
) {
    if let Some(p) = win.cursor_position() {
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
            if dt < DOUBLE_CLICK_DURATION {
                events.send(InteractionEvent::DoubleClick);
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
