use crate::scenes::{StaticSpriteDescriptor, TextLabel};
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use starling::aabb::AABB;

pub struct Canvas<'w, 's> {
    pub gizmos: Gizmos<'w, 's>,
    pub text_labels: Vec<TextLabel>,
    pub sprites: Vec<StaticSpriteDescriptor>,
    z_index: f32,
}

impl<'w, 's> Canvas<'w, 's> {
    pub fn new(gizmos: Gizmos<'w, 's>) -> Self {
        Self {
            gizmos,
            text_labels: Vec::new(),
            sprites: Vec::new(),
            z_index: 0.0,
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

    pub fn sprite<'a>(
        &'a mut self,
        pos: Vec2,
        angle: f32,
        path: impl Into<String>,
        z_index: f32,
        screen_dims: Vec2,
    ) -> &'a mut StaticSpriteDescriptor {
        let sprite = StaticSpriteDescriptor::new(pos, angle, path.into(), screen_dims, z_index);

        // let aabb = AABB::new(pos, screen_dims);
        // let obb = OBB::new(aabb, angle);
        // draw_obb(&mut self.gizmos, &obb, WHITE);

        self.sprites.push(sprite);
        self.sprites
            .last_mut()
            .expect("Literally just pushed an element")
    }

    pub fn rect(&mut self, aabb: AABB, color: impl Into<Srgba>) {
        self.z_index += 1.0;
        self.sprite(aabb.center, 0.0, "error", self.z_index, aabb.span)
            .set_color(color);
    }
}
