use crate::canvas::Canvas;
use crate::game::GameState;
use crate::onclick::OnClick;
use crate::z_index::ZOrdering;
use bevy::color::palettes::css::*;
use bevy::math::Vec2;
use bevy::prelude::*;
pub use bevy::sprite::Anchor;

#[derive(Debug, Clone)]
pub struct TextLabel {
    text: String,
    pos: Vec2,
    size: f32,
    color: Srgba,
    anchor: Anchor,
    z_index: ZOrdering,
}

impl TextLabel {
    pub fn new(text: String, pos: Vec2, size: f32) -> Self {
        Self {
            text,
            pos,
            size,
            color: WHITE,
            anchor: Anchor::Center,
            z_index: ZOrdering::Text,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn pos(&self) -> Vec2 {
        self.pos
    }

    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn color(&self) -> Srgba {
        self.color
    }

    pub fn anchor(&self) -> Anchor {
        self.anchor
    }

    pub fn z_order(&self) -> ZOrdering {
        self.z_index
    }

    // color

    pub fn with_color(mut self, color: Srgba) -> Self {
        self.color = color;
        self
    }

    pub fn set_color(&mut self, color: Srgba) -> &mut Self {
        self.color = color;
        self
    }

    // anchor

    pub fn set_anchor(&mut self, anchor: Anchor) -> &mut Self {
        self.anchor = anchor;
        self
    }

    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    // z ordering

    pub fn set_z_order(&mut self, z: ZOrdering) -> &mut Self {
        self.z_index = z;
        self
    }

    pub fn with_z_order(mut self, z: ZOrdering) -> Self {
        self.z_index = z;
        self
    }
}

#[derive(Debug, Clone)]
pub struct StaticSpriteDescriptor {
    pub position: Vec2,
    pub angle: f32,
    pub path: String,
    pub dims: Vec2,
    pub z_index: ZOrdering,
    pub color: Option<Srgba>,
}

impl StaticSpriteDescriptor {
    pub fn new(position: Vec2, angle: f32, path: String, dims: Vec2, z_index: ZOrdering) -> Self {
        Self {
            position,
            angle,
            path,
            dims,
            z_index,
            color: None,
        }
    }

    pub fn with_color(mut self, color: impl Into<Srgba>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn set_color(&mut self, color: impl Into<Srgba>) {
        self.color = Some(color.into());
    }
}

pub trait Render {
    fn draw(_canvas: &mut Canvas, _state: &GameState) -> Option<()> {
        None
    }
}
