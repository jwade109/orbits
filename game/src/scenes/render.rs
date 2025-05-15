use crate::planetary::GameState;
use bevy::color::Srgba;
use starling::math::Vec2;

pub struct TextLabel {
    pub text: String,
    pub position: Vec2,
    pub size: f32,
}

impl TextLabel {
    pub fn new(text: String, position: Vec2, size: f32) -> Self {
        Self {
            text,
            position,
            size,
        }
    }
}

pub struct StaticSpriteDescriptor {
    pub position: Vec2,
    pub path: String,
    pub scale: f32,
    pub z_index: f32,
}

pub trait Render {
    fn text_labels(state: &GameState) -> Vec<TextLabel>;

    fn sprites(state: &GameState) -> Vec<StaticSpriteDescriptor>;

    fn background_color(state: &GameState) -> Srgba;
}
