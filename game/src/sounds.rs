use crate::game::GameState;
use bevy::prelude::*;

pub fn sound_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<GameState>,
) {
    if state.button_was_pressed {
        match std::fs::canonicalize(state.args.audio_dir().join("button-up.ogg")) {
            Ok(path) => _ = commands.spawn((AudioPlayer::new(asset_server.load(path)),)),
            Err(e) => _ = error!("Failed to play sound: {}", e),
        }
        state.button_was_pressed = false;
    }
}
