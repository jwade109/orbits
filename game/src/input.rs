use crate::game::GameState;
use crate::scenes::CameraProjection;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use starling::nanotime::Nanotime;
use starling::prelude::AABB;

const DOUBLE_CLICK_DURATION: Nanotime = Nanotime::millis(300);

#[derive(Debug, Clone, Copy)]
struct MouseFrame {
    frame_no: u64,
    screen_pos: Vec2,
    wall_time: Nanotime,
}

impl MouseFrame {
    fn age(&self, wall_time: Nanotime) -> Nanotime {
        wall_time - self.wall_time
    }
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

#[derive(Debug, Default)]
enum ScrollDir {
    #[default]
    None,
    Up,
    Down,
}

#[derive(Debug, Default)]
pub struct InputState {
    frame_no: u64,

    hover: CursorTravel,
    left: CursorTravel,
    right: CursorTravel,
    middle: CursorTravel,

    on_double_click: Option<Vec2>,
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

    pub fn age(&self, button: MouseButt, order: FrameId, wall_time: Nanotime) -> Option<Nanotime> {
        let state = self.get_state(button);
        let frame = state.frame(order)?;
        Some(wall_time - frame.wall_time)
    }

    pub fn set_buttons(&mut self, buttons: ButtonInput<KeyCode>) {
        self.buttons = buttons;
    }

    pub fn set_scroll(&mut self, mut scroll: EventReader<MouseWheel>) {
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

    pub fn double_click(&self) -> Option<Vec2> {
        self.on_double_click
    }

    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.buttons.pressed(key)
    }

    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.buttons.just_pressed(key)
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

    pub fn world_position(
        &self,
        button: MouseButt,
        order: FrameId,
        ctx: &impl CameraProjection,
    ) -> Option<Vec2> {
        let p = self.position(button, order)?;
        Some(ctx.c2w(p))
    }
}

pub fn update_input_state(
    win: Single<&Window>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut evr_kbd: EventReader<KeyboardInput>,
    mut state: ResMut<GameState>,
) {
    let dims = Vec2::new(win.width(), win.height());
    let t = state.wall_time;

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
            wall_time: t,
        }
    } else {
        state.input.hover.set_up();
        state.input.left.set_up();
        state.input.right.set_up();
        state.input.middle.set_up();
        return;
    };

    state.input.hover.set_down(current_frame);
    state.input.on_double_click = None;
    state.input.on_mouse_left_up = true;

    if buttons.pressed(MouseButton::Left) {
        let age = state.input.left.down().map(|f| f.age(t));
        if state.input.left.up().is_some() {
            if let Some(age) = age {
                if age < DOUBLE_CLICK_DURATION {
                    state.input.on_double_click =
                        Some(current_frame.screen_pos - state.input.screen_bounds.span / 2.0);
                }
            }
        }
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
