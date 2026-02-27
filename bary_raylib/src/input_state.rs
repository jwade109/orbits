pub use rdev::Key;
use std::collections::HashSet;

#[derive(Debug, Default, Clone)]
pub struct InputState {
    buttons: HashSet<Key>,
}

impl InputState {
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.buttons.contains(&key)
    }

    pub fn set_pressed(&mut self, key: Key) {
        self.buttons.insert(key);
    }

    pub fn set_released(&mut self, key: Key) {
        self.buttons.remove(&key);
    }
}
