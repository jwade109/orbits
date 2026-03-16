use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum SoundEffect {
    Ping,
    Crossword,
    Open,
    Close,
    Follow,
    LeaveFollow,
    MouseoverPart,
    DestroyPart,
    SetWaypoint,
    GenericFailure,
    OpenEditor,
    LeaveEditor,
}

impl SoundEffect {
    pub fn to_path(&self) -> &'static str {
        get_sound_effect_asset_path(*self)
    }
}

pub type SoundEffects = Vec<SoundEffect>;

pub fn get_sound_effect_asset_path(effect: SoundEffect) -> &'static str {
    match effect {
        SoundEffect::Ping => "assets/sfx/ping.wav",
        SoundEffect::Crossword => "assets/sfx/nyt-crossword.ogg",
        SoundEffect::Open => "assets/sfx/soft-sine-open.wav",
        SoundEffect::Close => "assets/sfx/soft-sine-close.wav",
        SoundEffect::Follow => "assets/sfx/follow.wav",
        SoundEffect::LeaveFollow => "assets/sfx/leave-follow.wav",
        SoundEffect::MouseoverPart => "assets/sfx/mouseover-part.wav",
        SoundEffect::DestroyPart => "assets/sfx/destroy-part.wav",
        SoundEffect::SetWaypoint => "assets/sfx/soft-pulse.ogg",
        SoundEffect::GenericFailure => "assets/sfx/generic-failure.wav",
        SoundEffect::OpenEditor => "assets/sfx/open-editor.wav",
        SoundEffect::LeaveEditor => "assets/sfx/leave-editor.wav",
    }
}
