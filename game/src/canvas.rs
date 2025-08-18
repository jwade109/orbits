use crate::z_index::ZOrdering;
use crate::{
    drawing::draw_square,
    scenes::{StaticSpriteDescriptor, TextLabel},
};
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use starling::aabb::AABB;

pub struct Canvas<'w, 's> {
    pub gizmos: Gizmos<'w, 's>,
    pub text_labels: Vec<TextLabel>,
    pub sprites: Vec<StaticSpriteDescriptor>,
    pub painter: ShapePainter<'w, 's>,
}

impl<'w, 's> Canvas<'w, 's> {
    pub fn new(gizmos: Gizmos<'w, 's>, painter: ShapePainter<'w, 's>) -> Self {
        Self {
            gizmos,
            text_labels: Vec::new(),
            sprites: Vec::new(),
            painter,
        }
    }

    pub fn circle<'a>(&'a mut self, p: Vec2, radius: f32, color: Srgba) {
        self.painter.reset();
        self.painter.set_translation(p.extend(0.0));
        self.painter.set_color(color);
        self.painter.hollow = true;
        self.painter.thickness = 3.0;
        self.painter.circle(radius);
    }

    pub fn square(&mut self, p: Vec2, sidelength: f32, color: Srgba) {
        draw_square(&mut self.gizmos, p, sidelength, color);
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
        z_index: ZOrdering,
        screen_dims: Vec2,
    ) -> &'a mut StaticSpriteDescriptor {
        let sprite = StaticSpriteDescriptor::new(pos, angle, path.into(), screen_dims, z_index);

        self.sprites.push(sprite);
        self.sprites
            .last_mut()
            .expect("Literally just pushed an element")
    }

    pub fn rect<'a>(
        &'a mut self,
        aabb: AABB,
        z_index: ZOrdering,
        color: impl Into<Srgba>,
    ) -> &'a mut StaticSpriteDescriptor {
        let s = self.sprite(aabb.center, 0.0, "error", z_index, aabb.span);
        s.set_color(color.into());
        s
    }
}
