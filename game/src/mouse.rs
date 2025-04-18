use crate::planetary::GameState;
use crate::ui::InteractionEvent;
use bevy::prelude::*;
use core::time::Duration;
use starling::nanotime::Nanotime;
use starling::prelude::AABB;

const DOUBLE_CLICK_DURATION: Nanotime = Nanotime::millis(400);

#[derive(Debug, Clone, Copy)]
pub struct MousePos {
    frame_no: u32,
    screen_pos: Vec2,
    wall_time: Nanotime,
}

impl MousePos {
    fn new(frame_no: u32, screen_pos: Vec2, wall_time: Nanotime) -> Self {
        Self {
            frame_no,
            screen_pos,
            wall_time,
        }
    }
}

#[derive(Debug, Default)]
pub struct MouseState {
    last_click: Option<Nanotime>,
    current: Option<MousePos>,
    left_click: Option<MousePos>,
    right_click: Option<MousePos>,
    middle_click: Option<MousePos>,

    pub viewport_bounds: AABB,
    pub world_bounds: AABB,
    pub scale: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButtonSelect {
    Left,
    Right,
    Middle,
    Current,
}

impl MouseState {
    pub fn scale(&self) -> f32 {
        self.scale
    }

    #[deprecated]
    pub fn current(&self) -> Option<Vec2> {
        Some(self.current?.screen_pos)
    }

    #[deprecated]
    pub fn left(&self) -> Option<Vec2> {
        Some(self.left_click?.screen_pos)
    }

    #[deprecated]
    pub fn right(&self) -> Option<Vec2> {
        Some(self.right_click?.screen_pos)
    }

    #[deprecated]
    pub fn middle(&self) -> Option<Vec2> {
        Some(self.middle_click?.screen_pos)
    }

    pub fn position(&self, button: MouseButtonSelect) -> Option<Vec2> {
        let state = self.get_state(button)?;
        Some(state.screen_pos)
    }

    pub fn age(&self, button: MouseButtonSelect, wall_time: Nanotime) -> Option<Nanotime> {
        let state = self.get_state(button)?;
        Some(wall_time - state.wall_time)
    }

    fn get_state(&self, button: MouseButtonSelect) -> Option<&MousePos> {
        match button {
            MouseButtonSelect::Left => self.left_click.as_ref(),
            MouseButtonSelect::Right => self.right_click.as_ref(),
            MouseButtonSelect::Middle => self.middle_click.as_ref(),
            MouseButtonSelect::Current => self.current.as_ref(),
        }
    }

    pub fn on_click(&self, button: MouseButtonSelect, frame_no: u32) -> Option<Vec2> {
        let state = self.get_state(button)?;
        (state.frame_no == frame_no).then(|| state.screen_pos)
    }

    fn viewport_to_world(&self, p: Vec2) -> Vec2 {
        self.viewport_bounds.map(self.world_bounds, p)
    }

    pub fn world_position(&self, button: MouseButtonSelect) -> Option<Vec2> {
        let p = self.position(button)?;
        Some(self.viewport_to_world(p))
    }

    #[deprecated]
    pub fn current_world(&self) -> Option<Vec2> {
        Some(self.viewport_to_world(self.current()?))
    }

    #[deprecated]
    pub fn left_world(&self) -> Option<Vec2> {
        Some(self.viewport_to_world(self.left()?))
    }

    #[deprecated]
    pub fn right_world(&self) -> Option<Vec2> {
        Some(self.viewport_to_world(self.right()?))
    }

    #[deprecated]
    pub fn middle_world(&self) -> Option<Vec2> {
        Some(self.viewport_to_world(self.middle()?))
    }
}

pub fn update_mouse_state(
    win: Single<&Window>,
    buttons: Res<ButtonInput<MouseButton>>,
    camera: Single<&Transform, With<crate::planetary::SoftController>>,
    mut state: ResMut<GameState>,
    mut events: EventWriter<InteractionEvent>,
) {
    let dims = Vec2::new(win.width(), win.height());
    let t = state.wall_time;
    let f = state.current_frame_no;

    state.mouse.viewport_bounds = AABB::new(dims / 2.0, dims);
    state.mouse.world_bounds = AABB::new(camera.translation.xy(), dims * camera.scale.z);
    state.mouse.scale = camera.scale.z;

    let mp = if let Some(p) = win.cursor_position() {
        let p = Vec2::new(p.x, dims.y - p.y);
        MousePos::new(f, p, t)
    } else {
        state.mouse.current = None;
        state.mouse.left_click = None;
        state.mouse.right_click = None;
        state.mouse.middle_click = None;
        return;
    };

    state.mouse.current = Some(mp);

    if buttons.just_pressed(MouseButton::Left) {
        if let Some(l) = state.mouse.last_click {
            let dt = t - l;
            if dt < DOUBLE_CLICK_DURATION {
                events.send(InteractionEvent::DoubleClick(mp.screen_pos));
                state.mouse.last_click = None;
            }
        }
        state.mouse.last_click = Some(t);
        state.mouse.left_click = Some(mp);
    } else if buttons.just_released(MouseButton::Left) {
        state.mouse.left_click = None;
    }

    if buttons.just_pressed(MouseButton::Right) {
        state.mouse.right_click = Some(mp);
    } else if buttons.just_released(MouseButton::Right) {
        state.mouse.right_click = None;
    }

    if buttons.just_pressed(MouseButton::Middle) {
        state.mouse.middle_click = Some(mp);
    } else if buttons.just_released(MouseButton::Middle) {
        state.mouse.middle_click = None;
    }
}
