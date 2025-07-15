use enum_iterator::Sequence;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Sequence)]
pub enum Key {
    A,
    B,
    C,
    D,
}

impl Key {
    pub fn every() -> impl Iterator<Item = Key> {
        enum_iterator::all::<Key>()
    }
}

struct ButtonState {
    is_pressed: bool,
    just_pressed: bool,
    just_released: bool,
    is_eaten: bool,
}

impl ButtonState {
    fn new() -> Self {
        ButtonState {
            is_pressed: false,
            just_pressed: false,
            just_released: false,
            is_eaten: false,
        }
    }

    fn update(&mut self, pressed: bool) {
        let prev_int = self.is_pressed as u8;
        let cur_int = pressed as u8;
        *self = ButtonState {
            is_pressed: pressed,
            just_pressed: cur_int > prev_int,
            just_released: cur_int < prev_int,
            is_eaten: false,
        };
    }
}

pub struct KeyboardInput {
    buttons: HashMap<Key, ButtonState>,
}

impl KeyboardInput {
    pub fn new() -> Self {
        Self {
            buttons: Key::every().map(|k| (k, ButtonState::new())).collect(),
        }
    }

    pub fn update_keys(&mut self, keys: &HashSet<Key>) {
        for key in Key::every() {
            let state = keys.contains(&key);
            self.set_button(key, state);
        }
    }

    fn set_button(&mut self, key: Key, state: bool) {
        let s = self.buttons.get_mut(&key).unwrap();
        s.update(state);
    }

    fn eat_state(&mut self, key: Key) -> Option<&ButtonState> {
        let s = self.buttons.get_mut(&key).unwrap();
        if s.is_eaten {
            None
        } else {
            s.is_eaten = true;
            Some(s)
        }
    }

    pub fn eat_pressed(&mut self, key: Key) -> bool {
        if let Some(s) = self.eat_state(key) {
            s.is_pressed
        } else {
            false
        }
    }

    pub fn eat_just_pressed(&mut self, key: Key) -> bool {
        if let Some(s) = self.eat_state(key) {
            s.just_pressed
        } else {
            false
        }
    }

    pub fn eat_just_released(&mut self, key: Key) -> bool {
        if let Some(s) = self.eat_state(key) {
            s.just_released
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_input_system() {
        let mut input = KeyboardInput::new();

        let pressed_buttons = HashSet::from([Key::A, Key::B]);
        input.update_keys(&pressed_buttons);

        assert!(input.eat_pressed(Key::A));
        assert!(input.eat_just_pressed(Key::B));

        assert!(!input.eat_just_pressed(Key::B));
        assert!(!input.eat_just_released(Key::D));
        assert!(!input.eat_pressed(Key::A));

        let pressed_keys = HashSet::from([Key::D, Key::A]);
        input.update_keys(&pressed_keys);

        assert!(input.eat_just_pressed(Key::D));
        assert!(!input.eat_just_pressed(Key::D));
    }
}
