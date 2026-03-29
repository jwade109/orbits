pub use rdev::Key;
use std::collections::HashSet;

#[derive(Debug, Default, Clone)]
pub struct InputState {
    currently_pressed: HashSet<Key>,
    just_pressed: HashSet<Key>,
    just_pressed_debounced: HashSet<Key>,
    just_released: HashSet<Key>,
}

impl InputState {
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.currently_pressed.contains(&key)
    }

    pub fn just_pressed_debounced(&self, key: Key) -> bool {
        self.just_pressed_debounced.contains(&key)
    }

    pub fn just_released(&self, key: Key) -> bool {
        self.just_released.contains(&key)
    }

    pub fn set_pressed(&mut self, key: Key) {
        if !self.currently_pressed.contains(&key) {
            self.just_pressed_debounced.insert(key);
        }
        self.currently_pressed.insert(key);
        self.just_pressed.insert(key);
    }

    pub fn set_released(&mut self, key: Key) {
        self.currently_pressed.remove(&key);
        self.just_released.insert(key);
    }

    pub fn on_frame_boundary(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.just_pressed_debounced.clear();
    }
}
