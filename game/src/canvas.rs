use crate::starling::aabb::AABB;
use crate::z_index::ZOrdering;
use crate::{
    drawing::draw_square,
    scenes::{StaticSpriteDescriptor, TextLabel},
};
use bevy::prelude::*;
pub use bevy_vector_shapes::prelude::*;

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

    pub fn circle<'a>(&'a mut self, p: Vec3, radius: f32, color: Srgba) {
        self.painter.reset();
        self.painter.set_translation(p);
        self.painter.set_color(color);
        self.painter.hollow = true;
        self.painter.thickness = 3.0;
        self.painter.circle(radius);
    }

    pub fn fill_circle<'a>(&'a mut self, p: Vec3, radius: f32, color: Srgba) {
        self.painter.reset();
        self.painter.set_translation(p);
        self.painter.set_color(color);
        self.painter.hollow = false;
        self.painter.circle(radius);
    }

    pub fn line(&mut self, p: Vec2, q: Vec2, z: ZOrdering, color: Srgba) {
        self.painter.reset();
        self.painter.set_translation(Vec3::Z * z.as_f32());
        self.painter.set_color(color);
        self.painter.thickness = 3.0;
        self.painter.line(p.extend(0.0), q.extend(0.0));
    }

    pub fn line_t(&mut self, p: Vec2, q: Vec2, z: ZOrdering, t: f32, color: Srgba) {
        self.painter.reset();
        self.painter.set_translation(Vec3::Z * z.as_f32());
        self.painter.set_color(color);
        self.painter.thickness = t;
        self.painter.thickness_type = ThicknessType::World;
        self.painter.line(p.extend(0.0), q.extend(0.0));
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

    pub fn rect(&mut self, aabb: AABB, z_index: ZOrdering, color: impl Into<Srgba>) {
        self.painter.reset();
        self.painter
            .set_translation(aabb.center.extend(z_index.as_f32()));
        self.painter.set_color(color.into());
        self.painter.rect(aabb.span);
    }

    pub fn hollow_rect(&mut self, aabb: AABB, z: ZOrdering, c: impl Into<Srgba>, t: f32) {
        self.painter.reset();
        self.painter.hollow = true;
        self.painter.thickness = t;
        self.painter.set_translation(aabb.center.extend(z.as_f32()));
        self.painter.set_color(c.into());
        self.painter.rect(aabb.span);
    }
}
