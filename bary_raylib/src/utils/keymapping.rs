use crate::utils::{InputState, KB};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

pub struct KeyMapping<T>(HashMap<rdev::Key, T>);

impl<T: Copy> KeyMapping<T> {
    pub fn new(map: impl Into<HashMap<rdev::Key, T>>) -> Self {
        Self(map.into())
    }

    pub fn map(&self, key: rdev::Key) -> Option<T> {
        self.0.get(&key).copied()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ActionSet<T: Copy + Eq + Hash> {
    currently_pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    just_pressed_debounced: HashSet<T>,
    just_released: HashSet<T>,
}

fn apply_keymapping<T: Copy + Eq + Hash>(
    mapping: &KeyMapping<T>,
    keys: impl Iterator<Item = KB>,
) -> HashSet<T> {
    keys.filter_map(|e| {
        if let KB::Key(e) = e {
            mapping.map(e)
        } else {
            None
        }
    })
    .collect()
}

impl<T: Copy + Eq + Hash> ActionSet<T> {
    pub fn new(input: &InputState, map: &KeyMapping<T>) -> Self {
        Self {
            currently_pressed: apply_keymapping(map, input.get_currently_pressed()),
            just_pressed: apply_keymapping(map, input.get_just_pressed()),
            just_pressed_debounced: apply_keymapping(map, input.get_just_pressed_debounced()),
            just_released: apply_keymapping(map, input.get_just_released()),
        }
    }

    pub fn is_active(&self, key: T) -> bool {
        let key = key.into();
        self.currently_pressed.contains(&key)
    }

    pub fn just_pressed_debounced(&self, key: T) -> bool {
        let key = key.into();
        self.just_pressed_debounced.contains(&key)
    }

    pub fn just_released(&self, key: T) -> bool {
        let key = key.into();
        self.just_released.contains(&key)
    }

    pub fn just_triggered(&self, key: T) -> bool {
        let key = key.into();
        self.just_pressed.contains(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum DrivingAction {
        DriveLeft,
        DriveRight,
        DriveForward,
        DriveBackward,
    }

    #[test]
    fn action_sets() {
        use rdev::Key::*;

        // define a driving action set -
        // this would be in a config file somewhere
        let driving = KeyMapping::<DrivingAction>::new([
            (KeyW, DrivingAction::DriveForward),
            (KeyA, DrivingAction::DriveLeft),
            (KeyS, DrivingAction::DriveBackward),
            (KeyD, DrivingAction::DriveRight),
        ]);

        let mut input = InputState::default();

        // ==================================================
        // frame 0 -- KeyD is just pushed
        // ==================================================

        input.set_pressed(KeyD);

        let actions = ActionSet::new(&input, &driving);

        assert!(input.just_pressed(KeyD));

        assert!(actions.just_triggered(DrivingAction::DriveRight));
        assert!(!actions.just_triggered(DrivingAction::DriveBackward));
        assert!(!actions.just_triggered(DrivingAction::DriveLeft));
        assert!(!actions.just_triggered(DrivingAction::DriveForward));

        assert!(input.is_key_pressed(KeyD));

        assert!(actions.is_active(DrivingAction::DriveRight));
        assert!(!actions.is_active(DrivingAction::DriveBackward));
        assert!(!actions.is_active(DrivingAction::DriveLeft));
        assert!(!actions.is_active(DrivingAction::DriveForward));

        input.on_frame_boundary();

        // ==================================================
        // frame 1 -- nothing happens. KeyD is still held down
        // ==================================================

        let actions = ActionSet::new(&input, &driving);

        assert!(!input.just_pressed(KeyD));
        assert!(input.is_key_pressed(KeyD));

        // nothing has just been triggered
        assert!(!actions.just_triggered(DrivingAction::DriveRight));
        assert!(!actions.just_triggered(DrivingAction::DriveBackward));
        assert!(!actions.just_triggered(DrivingAction::DriveLeft));
        assert!(!actions.just_triggered(DrivingAction::DriveForward));

        // only drive right is still active
        assert!(actions.is_active(DrivingAction::DriveRight));
        assert!(!actions.is_active(DrivingAction::DriveBackward));
        assert!(!actions.is_active(DrivingAction::DriveLeft));
        assert!(!actions.is_active(DrivingAction::DriveForward));

        input.on_frame_boundary();

        // ==================================================
        // frame 2 -- KeyD released, F and S pressed
        // ==================================================

        input.set_released(KeyD);
        input.set_pressed(KeyF);
        input.set_pressed(KeyS);
        input.set_pressed(KeyW);

        assert!(input.just_released(KeyD));

        assert!(input.just_pressed(KeyF));
        assert!(input.just_pressed(KeyS));
        assert!(input.just_pressed(KeyW));

        let actions = ActionSet::new(&input, &driving);

        // S and W trigger Forwards and Backwards
        assert!(!actions.just_triggered(DrivingAction::DriveRight));
        assert!(actions.just_triggered(DrivingAction::DriveBackward));
        assert!(!actions.just_triggered(DrivingAction::DriveLeft));
        assert!(actions.just_triggered(DrivingAction::DriveForward));

        // only those two are active
        assert!(!actions.is_active(DrivingAction::DriveRight));
        assert!(actions.is_active(DrivingAction::DriveBackward));
        assert!(!actions.is_active(DrivingAction::DriveLeft));
        assert!(actions.is_active(DrivingAction::DriveForward));

        // releasing D releases Right
        assert!(actions.just_released(DrivingAction::DriveRight));
        assert!(!actions.just_released(DrivingAction::DriveBackward));
        assert!(!actions.just_released(DrivingAction::DriveLeft));
        assert!(!actions.just_released(DrivingAction::DriveForward));
    }
}
