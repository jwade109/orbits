use crate::scenes::{StaticSpriteDescriptor, TextLabel};
use bevy::color::palettes::css::*;
use bevy::prelude::*;

pub struct Canvas<'w, 's> {
    pub gizmos: Gizmos<'w, 's>,
    pub text_labels: Vec<TextLabel>,
    pub sprites: Vec<StaticSpriteDescriptor>,
}

impl<'w, 's> Canvas<'w, 's> {
    pub fn new(gizmos: Gizmos<'w, 's>) -> Self {
        Self {
            gizmos,
            text_labels: Vec::new(),
            sprites: Vec::new(),
        }
    }

    pub fn circle(&mut self) {
        self.gizmos.circle_2d(Isometry2d::IDENTITY, 40.0, WHITE);
    }

    pub fn label(&mut self, label: TextLabel) {
        self.text_labels.push(label);
    }

    pub fn text<'a>(
        &'a mut self,
        text: impl Into<String>,
        pos: Vec2,
        size: f32,
    ) -> &'a mut TextLabel {
        let label = TextLabel::new(text.into(), pos, size);
        self.text_labels.push(label);
        self.text_labels
            .last_mut()
            .expect("Literally just pushed an element")
    }

    pub fn file_sprite(
        &mut self,
        pos: Vec2,
        angle: f32,
        path: impl Into<String>,
        scale: f32,
        z_index: f32,
    ) {
        let sprite = StaticSpriteDescriptor::filesystem(pos, angle, path.into(), scale, z_index);
        self.sprites.push(sprite);
    }

    pub fn proc_sprite(
        &mut self,
        pos: Vec2,
        angle: f32,
        path: impl Into<String>,
        scale: f32,
        z_index: f32,
    ) {
        let sprite = StaticSpriteDescriptor::procedural(pos, angle, path.into(), scale, z_index);
        self.sprites.push(sprite);
    }
}
