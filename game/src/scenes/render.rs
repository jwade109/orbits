use crate::onclick::OnClick;
use crate::planetary::GameState;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use layout::layout::Tree;
use starling::math::Vec2;

pub struct TextLabel {
    pub text: String,
    pub position: Vec2,
    pub size: f32,
    color: Srgba,
}

impl TextLabel {
    pub fn new(text: String, position: Vec2, size: f32) -> Self {
        Self {
            text,
            position,
            size,
            color: WHITE,
        }
    }

    pub fn with_color(mut self, color: Srgba) -> Self {
        self.color = color;
        self
    }

    pub fn color(&self) -> Srgba {
        self.color
    }
}

pub struct StaticSpriteDescriptor {
    pub position: Vec2,
    pub angle: f32,
    pub path: String,
    pub scale: f32,
    pub z_index: f32,
    pub color: Option<Srgba>,
}

impl StaticSpriteDescriptor {
    pub fn new(position: Vec2, angle: f32, path: String, scale: f32, z_index: f32) -> Self {
        Self {
            position,
            angle,
            path,
            scale,
            z_index,
            color: None,
        }
    }

    pub fn with_color(mut self, color: Srgba) -> Self {
        self.color = Some(color);
        self
    }
}

pub trait Render {
    fn text_labels(state: &GameState) -> Option<Vec<TextLabel>>;

    fn sprites(state: &GameState) -> Option<Vec<StaticSpriteDescriptor>>;

    fn background_color(state: &GameState) -> Srgba;

    fn draw_gizmos(gizmos: &mut Gizmos, state: &GameState) -> Option<()>;

    fn ui(state: &GameState) -> Option<Tree<OnClick>>;
}
