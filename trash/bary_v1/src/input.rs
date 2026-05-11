use crate::game::GameState;
use bary_core::prelude::AABB;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy)]
struct MouseFrame {
    frame_no: u64,
    screen_pos: Vec2,
}

#[derive(Default, Debug, Clone, Copy)]
enum CursorTravel {
    #[default]
    None,
    Traveling(MouseFrame, MouseFrame),
    Finished(MouseFrame, MouseFrame),
}

impl CursorTravel {
    fn set_down(&mut self, current_frame: MouseFrame) {
        let next = match self {
            Self::None => Self::Traveling(current_frame, current_frame),
            Self::Traveling(down, _) => Self::Traveling(*down, current_frame),
            Self::Finished(_, _) => Self::Traveling(current_frame, current_frame),
        };

        *self = next;
    }

    fn set_up(&mut self) {
        let down = match self.down() {
            Some(d) => d,
            None => return,
        };
        let up = match self.current() {
            Some(d) => d,
            None => return,
        };

        *self = Self::Finished(*down, *up);
    }

    fn down(&self) -> Option<&MouseFrame> {
        match &self {
            Self::None => None,
            Self::Traveling(f, _) | Self::Finished(f, _) => Some(f),
        }
    }

    fn current(&self) -> Option<&MouseFrame> {
        match &self {
            Self::Traveling(_, c) => Some(c),
            _ => None,
        }
    }

    fn up(&self) -> Option<&MouseFrame> {
        match &self {
            Self::Finished(_, f) => Some(f),
            _ => None,
        }
    }

    fn frame(&self, order: FrameId) -> Option<&MouseFrame> {
        match order {
            FrameId::Current => self.current(),
            FrameId::Down => self.down(),
            FrameId::Up => self.up(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
enum ScrollDir {
    #[default]
    None,
    Up,
    Down,
}

#[derive(Debug, Default, Clone)]
pub struct InputState {
    frame_no: u64,

    hover: CursorTravel,
    left: CursorTravel,
    right: CursorTravel,
    middle: CursorTravel,

    on_mouse_left_up: bool,

    pub screen_bounds: AABB,

    buttons: ButtonInput<KeyCode>,
    pub keyboard_events: Vec<KeyboardInput>,
    scroll: ScrollDir,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButt {
    Hover,
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy)]
pub enum FrameId {
    Down,
    Current,
    Up,
}

impl InputState {
    /// Position of the mouse in camera-screen space.
    ///
    /// (0, 0) is always the center of the screen.
    /// (-w/2, -h/2) is the bottom left corner, (w/2, h/2) is the top right corner.
    pub fn position(&self, button: MouseButt, order: FrameId) -> Option<Vec2> {
        let state = self.get_state(button);
        let frame = state.frame(order)?;
        Some(frame.screen_pos - self.screen_bounds.span / 2.0)
    }

    pub fn current(&self) -> Option<Vec2> {
        self.position(MouseButt::Hover, FrameId::Current)
    }

    pub fn set_buttons(&mut self, buttons: ButtonInput<KeyCode>) {
        self.buttons = buttons;
    }

    pub fn set_scroll(&mut self, mut scroll: MessageReader<MouseWheel>) {
        self.scroll = match scroll.read().next() {
            Some(m) => match m.y.partial_cmp(&0.0) {
                None => ScrollDir::None,
                Some(std::cmp::Ordering::Equal) => ScrollDir::None,
                Some(std::cmp::Ordering::Greater) => ScrollDir::Up,
                Some(std::cmp::Ordering::Less) => ScrollDir::Down,
            },
            None => ScrollDir::None,
        }
    }

    pub fn is_scroll_down(&self) -> bool {
        match self.scroll {
            ScrollDir::Down => true,
            _ => false,
        }
    }

    pub fn is_scroll_up(&self) -> bool {
        match self.scroll {
            ScrollDir::Up => true,
            _ => false,
        }
    }

    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.buttons.pressed(key)
    }

    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.buttons.just_pressed(key)
    }

    pub fn pressed(&self) -> impl Iterator<Item = &KeyCode> {
        self.buttons.get_pressed()
    }

    fn get_state(&self, button: MouseButt) -> &CursorTravel {
        match button {
            MouseButt::Hover => &self.hover,
            MouseButt::Left => &self.left,
            MouseButt::Right => &self.right,
            MouseButt::Middle => &self.middle,
        }
    }

    pub fn on_frame(&self, button: MouseButt, order: FrameId) -> Option<Vec2> {
        let delta = match order {
            FrameId::Current => 0,
            FrameId::Down => 1,
            FrameId::Up => 1,
        };
        let state = self.get_state(button);
        let frame = state.frame(order)?;
        (self.frame_no == frame.frame_no + delta)
            .then(|| frame.screen_pos - self.screen_bounds.span / 2.0)
    }
}

pub fn update_input_state(
    win: Single<&Window>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut evr_kbd: MessageReader<KeyboardInput>,
    mut state: ResMut<GameState>,
) {
    let dims = Vec2::new(win.width(), win.height());

    state.input.keyboard_events.clear();
    for event in evr_kbd.read() {
        state.input.keyboard_events.push(event.clone());
    }

    state.input.frame_no += 1;
    state.input.screen_bounds = AABB::new(dims / 2.0, dims);

    let current_frame = if let Some(p) = win.cursor_position() {
        let p = Vec2::new(p.x, dims.y - p.y);
        MouseFrame {
            frame_no: state.input.frame_no,
            screen_pos: p,
        }
    } else {
        state.input.hover.set_up();
        state.input.left.set_up();
        state.input.right.set_up();
        state.input.middle.set_up();
        return;
    };

    state.input.hover.set_down(current_frame);
    state.input.on_mouse_left_up = true;

    if buttons.pressed(MouseButton::Left) {
        state.input.left.set_down(current_frame);
    } else {
        state.input.on_mouse_left_up = true;
        state.input.left.set_up();
    }

    if buttons.pressed(MouseButton::Right) {
        state.input.right.set_down(current_frame);
    } else {
        state.input.right.set_up();
    }

    if buttons.pressed(MouseButton::Middle) {
        state.input.middle.set_down(current_frame);
    } else {
        state.input.middle.set_up();
    }
}
