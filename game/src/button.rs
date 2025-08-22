use crate::prelude::*;
use bevy::color::palettes::css::*;
use bevy::color::*;
use starling::prelude::*;

#[derive(Debug, Clone)]
pub struct ExpandButton {
    pos: Vec2,
    dims: Vec2,
    text: String,
    sprite: String,
    animation: Lpf,
    is_hovered: bool,
    is_clicked: bool,
    onclick: OnClick,
}

impl ExpandButton {
    pub fn new(
        text: impl Into<String>,
        onclick: OnClick,
        pos: Vec2,
        dims: Vec2,
        sprite: impl Into<String>,
    ) -> Self {
        Self {
            pos,
            dims,
            text: text.into(),
            sprite: sprite.into(),
            animation: Lpf::new(0.0, 0.0, 0.3),
            is_hovered: false,
            is_clicked: false,
            onclick,
        }
    }

    pub fn inner_bounds(&self) -> AABB {
        AABB::from_arbitrary(self.pos, self.pos + self.dims)
    }

    pub fn label_bounds(&self) -> AABB {
        let low = self.pos + Vec2::new(self.dims.x + 10.0, 0.0);
        let width = (20.0 + self.text.len() as f32 * 18.0) * self.anim();
        AABB::from_arbitrary(low, low + Vec2::new(width, self.dims.y))
    }

    pub fn sprite(&self) -> &String {
        &self.sprite
    }

    pub fn anim(&self) -> f32 {
        self.animation.actual
    }
}

impl Interactive for ExpandButton {
    fn on_left_mouse_down(&mut self) -> Option<OnClick> {
        if !self.is_hovered {
            return None;
        }
        self.is_clicked = true;
        None
    }

    fn on_left_mouse_up(&mut self) -> Option<OnClick> {
        if self.is_clicked {
            self.is_clicked = false;
            Some(self.onclick.clone())
        } else {
            None
        }
    }

    fn on_mouse_move(&mut self, p: &mut Take<Vec2>) {
        self.is_hovered = p
            .peek()
            .map(|p| self.inner_bounds().contains(*p))
            .unwrap_or(false);
        if self.is_hovered {
            p.take();
        }
    }

    fn step(&mut self) {
        self.animation.target = self.is_hovered as u8 as f32;
        self.animation.step();
        if !self.is_hovered {
            self.is_clicked = false;
        }
    }
}

pub fn draw_button(canvas: &mut Canvas, button: &ExpandButton) {
    let alpha = lerp(0.03, 1.0, button.anim());
    let aabb = button.inner_bounds();

    let aabb = if button.is_clicked {
        aabb.offset(-Vec2::splat(2.0))
    } else {
        aabb
    };

    let color = if button.is_clicked { SLATE_BLUE } else { GRAY };

    canvas.rect(aabb, ZOrdering::Ui, color.with_alpha(alpha));
    canvas
        .sprite(aabb.center, 0.0, button.sprite(), ZOrdering::Ui2, aabb.span)
        .color = Some(WHITE.with_alpha(alpha.clamp(0.3, 1.0)));

    let aabb = button.label_bounds();
    canvas.rect(aabb, ZOrdering::Ui, TEAL.with_alpha(alpha));
    canvas
        .text(button.text.clone(), aabb.center, button.anim())
        .z_index = ZOrdering::Ui2;
}
