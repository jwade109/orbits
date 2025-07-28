use crate::game::GameState;
use bevy::audio::*;
use bevy::prelude::*;

pub fn sound_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<GameState>,
) {
    for (s, v, do_loop) in state.sounds.sounds() {
        let handle = match std::fs::canonicalize(state.args.audio_dir().join(s)) {
            Ok(path) => asset_server.load(path),
            Err(e) => {
                error!("Failed to play sound: {}", e);
                continue;
            }
        };
        let player = AudioPlayer::new(handle);
        let mut settings = PlaybackSettings::default().with_volume(Volume::new(v));
        if do_loop {
            settings.mode = PlaybackMode::Loop;
        }
        commands.spawn((player, settings));
    }
}

pub struct EnvironmentSounds {
    sounds: Vec<(String, f32, bool)>,
}

impl EnvironmentSounds {
    pub fn new() -> Self {
        Self { sounds: Vec::new() }
    }

    pub fn play_loop(&mut self, name: impl Into<String>, volume: f32) {
        self.sounds.push((name.into(), volume, true));
    }

    pub fn play_once(&mut self, name: impl Into<String>, volume: f32) {
        self.sounds.push((name.into(), volume, false));
    }

    pub fn sounds(&mut self) -> Vec<(String, f32, bool)> {
        let r = self.sounds.clone();
        self.sounds.clear();
        r
    }
}
