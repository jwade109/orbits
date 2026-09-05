use crate::event_bus::*;
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::track::*;
use kira::*;
use log::info;

pub struct SoundManager {
    _manager: AudioManager,
    track: TrackHandle,
    sounds: Vec<(String, StaticSoundHandle)>,
}

impl SoundManager {
    pub fn new() -> Self {
        let mut _manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
        let builder = TrackBuilder::new();
        let track = _manager.add_sub_track(builder).unwrap();

        Self {
            _manager,
            track,
            sounds: Vec::new(),
        }
    }

    pub fn update(&mut self) {
        if self.track.num_sounds() > 0 {
            info!("{} sounds playing", self.track.num_sounds());
        }

        self.sounds.retain(|(_name, handle)| {
            if handle.state() == PlaybackState::Stopped {
                false
            } else {
                true
            }
        });
    }

    fn play_sound(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("playing sound {path}");
        let sound = StaticSoundData::from_file(path)?;
        let mut handle = self.track.play(sound.clone())?;
        let pbr = bary_core::prelude::rand(0.92, 1.06) as f64;
        handle.set_playback_rate(pbr, kira::Tween::default());
        self.sounds.push((path.into(), handle));
        Ok(())
    }

    pub fn handle_events(&mut self, events: &EventBus) {
        for event in events.iter() {
            match event {
                TrainEvent::Sound => _ = self.play_sound("assets/sfx/button-up.ogg"),
                _ => (),
            }
        }
    }
}
