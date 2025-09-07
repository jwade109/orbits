use crate::game::GameState;
use bevy::audio::*;
use bevy::prelude::*;
use starling::prelude::*;

pub fn sound_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut music_controller: Query<(&mut AudioSink, &TrackTag)>,
    mut state: ResMut<GameState>,
) {
    for (s, v, do_loop, track) in state.sounds.sounds() {
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
        commands.spawn((player, settings, track));
    }

    let sv = state
        .piloting()
        .map(|id| state.universe.surface_vehicles.get(&id))
        .flatten();

    let current_vehicle_is_thrusting = {
        if let Some(sv) = sv {
            sv.vehicle.is_thrusting()
        } else {
            false
        }
    };

    let has_current = state.piloting().is_some();

    let (high, mids, bass) = match (state.paused, has_current, current_vehicle_is_thrusting) {
        (true, _, _) => (0.0, 0.0, 0.4),
        (_, false, _) => (0.0, 0.0, 0.7),
        (_, true, false) => (0.1, 0.5, 0.3),
        (_, true, true) => (0.8, 0.6, 0.4),
    };

    for (sink, track) in &mut music_controller {
        let (target_volume, rate) = match track {
            TrackTag::High => (high, 0.01),
            TrackTag::Mids => (mids, 0.01),
            TrackTag::Bass => (bass, 0.01),
            TrackTag::Thrust => (current_vehicle_is_thrusting as u8 as f32 * 0.6, 0.5),
            _ => continue,
        };

        let mut volume = sink.volume();
        volume += (target_volume - volume) * rate;
        sink.set_volume(volume);
    }
}

pub struct EnvironmentSounds {
    sounds: Vec<(String, f32, bool, TrackTag)>,
}

impl EnvironmentSounds {
    pub fn new() -> Self {
        Self { sounds: Vec::new() }
    }

    pub fn play_loop(&mut self, name: impl Into<String>, volume: f32, track: TrackTag) {
        self.sounds.push((name.into(), volume, true, track));
    }

    pub fn play_once(&mut self, name: impl Into<String>, volume: f32) {
        self.sounds
            .push((name.into(), volume, false, TrackTag::Sfx));
    }

    pub fn sounds(&mut self) -> Vec<(String, f32, bool, TrackTag)> {
        let r = self.sounds.clone();
        self.sounds.clear();
        r
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub enum TrackTag {
    Bass,
    Mids,
    High,
    Thrust,
    Sfx,
}

pub struct MultiVoiceTrack {}
