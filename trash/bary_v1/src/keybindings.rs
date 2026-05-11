use crate::game::GameState;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

pub fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    scroll: MessageReader<MouseWheel>,
) {
    state.input.set_buttons(keys.clone());
    state.input.set_scroll(scroll);
}
