use bevy::math::Vec2;
use starling::aabb::AABB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Idle,
    Hovered,
    Pressed,
}

#[derive(Debug, Clone)]
pub struct Button {
    state: ButtonState,
    aabb: AABB,
    debounce: bool,
    name: String,
    inner: bool,
}

impl Button {
    pub fn new(name: &str, p1: impl Into<Vec2>, p2: impl Into<Vec2>, debounce: bool) -> Self {
        Button {
            state: ButtonState::Idle,
            aabb: AABB::from_arbitrary(p1, p2),
            debounce,
            name: name.to_string(),
            inner: false,
        }
    }

    pub fn bounds(&self) -> &AABB {
        &self.aabb
    }

    pub fn state(&self) -> bool {
        self.inner
    }

    pub fn set(&mut self, state: bool) {
        self.inner = state;
    }

    pub fn button_state(&self) -> ButtonState {
        self.state
    }

    pub fn update(&mut self, pos: Vec2, clicked: bool) -> bool {
        let contains = self.aabb.contains(pos);
        let prev = self.state;
        self.state = match (clicked, contains, self.state) {
            (_, false, _) => ButtonState::Idle,
            (false, true, _) => ButtonState::Hovered,
            (true, true, ButtonState::Idle) => ButtonState::Idle,
            (true, true, ButtonState::Pressed) | (true, true, ButtonState::Hovered) => {
                ButtonState::Pressed
            }
        };

        let fire = match (prev, self.state, self.debounce) {
            (ButtonState::Pressed, ButtonState::Pressed, true) => false,
            (_, ButtonState::Pressed, _) => true,
            (_, _, _) => false,
        };

        if fire && self.debounce {
            self.inner = !self.inner;
            println!("{} toggled to {}", self.name, self.inner);
        } else if !self.debounce {
            self.inner = fire;
            println!("{} toggled to {}", self.name, self.inner);
        }
        contains
    }
}
