use crate::interactive::Interactive;
use crate::onclick::OnClick;
use crate::z_index::ZOrdering;
use bevy::color::palettes::css::*;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use starling::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonId {
    Editor,
    Rcs,
    Idle,
    Prograde,
    Retrograde,
    Position,
    Attitude,
    Launch,
}

pub struct TextButton {
    id: ButtonId,
    origin: Vec2,
    width: f32,
    text: String,
    state: bool,
    is_hovered: bool,
    is_clicked: bool,
    is_visible: bool,
}

const TEXT_HEIGHT: f32 = 30.0;

impl TextButton {
    pub fn new(origin: Vec2, text: impl Into<String>, id: ButtonId) -> Self {
        let text = text.into();
        let width = text.len() as f32 * 17.0;
        Self {
            id,
            origin,
            width,
            text,
            state: false,
            is_hovered: false,
            is_clicked: false,
            is_visible: true,
        }
    }

    pub fn bounds(&self) -> AABB {
        let other = self.origin + Vec2::new(self.width, -TEXT_HEIGHT);
        AABB::from_arbitrary(self.origin, other)
    }

    pub fn text_color(&self) -> Srgba {
        if self.state {
            let mix = if self.is_clicked && self.is_hovered {
                0.7
            } else if self.is_hovered {
                0.9
            } else {
                1.0
            };
            GRAY.mix(&WHITE, mix)
        } else {
            let mix = if self.is_clicked && self.is_hovered {
                0.7
            } else if self.is_hovered {
                0.4
            } else {
                0.0
            };
            GRAY.mix(&WHITE, mix)
        }
    }
}

impl Interactive for TextButton {
    fn on_left_mouse_down(&mut self) -> Option<OnClick> {
        if !self.is_visible {
            return None;
        }
        self.is_clicked = self.is_hovered;
        None
    }

    fn on_left_mouse_up(&mut self) -> Option<OnClick> {
        if !self.is_visible {
            return None;
        }
        let ret = if self.is_clicked && self.is_hovered {
            Some(OnClick::TextButtonClicked(self.id))
        } else {
            None
        };
        self.is_clicked = false;
        ret
    }

    fn on_mouse_move(&mut self, p: &mut Take<Vec2>) -> Option<OnClick> {
        if !self.is_visible {
            return None;
        }
        let bounds = self.bounds();
        if let Some(pos) = p.peek() {
            self.is_hovered = bounds.contains(*pos);
            if self.is_hovered {
                p.take();
            }
        }
        None
    }

    fn step(&mut self) -> Option<OnClick> {
        None
    }

    fn on_key(&mut self, _key: &KeyboardInput) -> Option<OnClick> {
        None
    }

    fn update(&mut self, facade: &crate::prelude::UiFacade) {
        self.is_visible = facade.is_piloting();
        self.state = facade.get_state(self.id);
        self.origin.y = -facade.get_screen_bounds().y / 2.0 + TEXT_HEIGHT + 10.0;
    }

    fn draw(&self, canvas: &mut crate::prelude::Canvas) {
        if !self.is_visible {
            return;
        }
        let z = ZOrdering::Ui;
        let text_color = self.text_color();
        let aabb = self.bounds();

        let n = if self.state { 4 } else { 1 };

        // TODO bootleg glowing effect
        for _ in 0..n {
            canvas.hollow_rect(aabb, z, text_color, 1.0);
            canvas
                .text(self.text.clone(), aabb.center, 0.73)
                .set_z_order(ZOrdering::Ui2)
                .set_color(text_color);
        }
    }
}
