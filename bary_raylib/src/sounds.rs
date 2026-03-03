use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum SoundEffect {
    Close,
    Crossword,
}

impl SoundEffect {
    pub fn to_path(&self) -> &'static str {
        get_sound_effect_asset_path(*self)
    }
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct SoundEffects {
    pub effects: Vec<SoundEffect>,
}

pub fn get_sound_effect_asset_path(effect: SoundEffect) -> &'static str {
    match effect {
        SoundEffect::Close => "assets/sfx/close-window.wav",
        SoundEffect::Crossword => "assets/sfx/nyt-crossword.ogg",
    }
}
