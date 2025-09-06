use crate::prelude::*;
pub use bevy::input::keyboard::{Key, KeyboardInput};
pub use bevy::input::mouse::MouseButton;
use starling::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum WindowClass {
    Tutorial(usize),
    Hello,
    CurrentVehicleInfo,
    VehicleInfo(EntityId),
}

impl WindowClass {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Tutorial(_) => "Tutorial",
            Self::Hello => "Hello!",
            Self::CurrentVehicleInfo => "Vehicle Info",
            Self::VehicleInfo(_) => "Vehicle Info",
        }
    }
}

#[derive(Debug)]
pub struct UiWindow {
    pub id: u32,
    pub class: WindowClass,
    pub title: String,
    pub contents: String,
    origin: Vec2,
    target_origin: Vec2,
    pub static_content_dims: Vec2,
    pub handle_height: f32,
    pub is_hovered: bool,
    pub is_clicked: bool,
    pub mouse_offset: Option<Vec2>,
    pub is_focused: bool,
    is_minimized: bool,
    collapse_lpf: Lpf,
    focus_animation: Lpf,
    hover_animation: Lpf,
}

impl UiWindow {
    pub fn new(id: u32, class: WindowClass) -> Self {
        let origin = randvec(100.0, 400.0);
        Self {
            id,
            class,
            title: class.title().into(),
            contents: String::new(),
            origin,
            target_origin: origin + randvec(50.0, 80.0),
            static_content_dims: Vec2::new(rand(350.0, 470.0), rand(200.0, 500.0)),
            handle_height: 30.0,
            is_hovered: false,
            is_clicked: false,
            mouse_offset: None,
            is_focused: false,
            is_minimized: false,
            collapse_lpf: Lpf::new(1.0, 0.0, 0.3),
            focus_animation: Lpf::new(0.0, 0.0, 0.3),
            hover_animation: Lpf::new(0.0, 0.0, 0.3),
        }
    }

    pub fn set_origin(&mut self, p: Vec2) {
        self.origin = p;
        self.target_origin = p;
    }

    pub fn set_target_pos(&mut self, p: Vec2) {
        self.target_origin = p;
    }

    pub fn static_content_bounds(&self) -> AABB {
        AABB::from_arbitrary(
            self.origin,
            self.origin + Vec2::new(self.static_content_dims.x, -self.static_content_dims.y),
        )
    }

    pub fn dynamic_content_bounds(&self) -> AABB {
        let cb = self.static_content_bounds();
        let top_left = cb.top_left();
        let top_right = cb.upper();
        let h = cb.span.y * self.collapse_lpf.actual;
        AABB::from_arbitrary(top_left - Vec2::Y * h, top_right)
    }

    pub fn buttons(&self) -> impl Iterator<Item = (AABB, OnClick, Srgba)> {
        let h = self.handle_height;
        let button_dims = Vec2::splat(h);
        let width = self.static_content_dims.x;
        let pad_size = 12.0;
        let button_aabb = AABB::from_arbitrary(Vec2::ZERO, button_dims);
        [
            (
                button_aabb.offset(Vec2::X * (width - h)).padded(-pad_size),
                OnClick::CloseWindow(self.id),
                RED,
            ),
            (
                button_aabb.offset(Vec2::X * (width - 2.0 * h)).padded(-pad_size),
                if self.is_minimized {
                    OnClick::MaximizeWindow(self.id)
                } else {
                    OnClick::MinimizeWindow(self.id)
                },
                GREEN,
            ),
        ]
        .into_iter()
    }

    pub fn handle_bounds(&self) -> AABB {
        let cb = self.static_content_bounds();
        let tl = cb.top_left();
        let tr = cb.upper();
        AABB::from_arbitrary(tl + Vec2::Y * self.handle_height, tr)
    }

    pub fn dynamic_total_bounds(&self) -> AABB {
        let content_bounds = self.dynamic_content_bounds();
        let handle_bounds = self.handle_bounds();
        AABB::from_arbitrary(content_bounds.lower(), handle_bounds.upper())
    }

    pub fn shadow(&self) -> AABB {
        let cb = self.dynamic_total_bounds();
        cb.offset(Vec2::splat(-7.0))
    }

    pub fn minimize(&mut self) {
        self.is_minimized = true;
        self.is_focused = false;
        self.is_clicked = false;
    }

    pub fn maximize(&mut self) {
        self.is_minimized = false;
    }
}

impl Interactive for UiWindow {
    fn on_left_mouse_down(&mut self) -> Option<OnClick> {
        if self.is_hovered {
            self.is_clicked = true;
            self.is_focused = true;
        } else {
            self.is_focused = false;
        }

        if let Some(off) = self.mouse_offset {
            for (bounds, event, _) in self.buttons() {
                if bounds.contains(off) {
                    return Some(event);
                }
            }
        }

        None
    }

    fn on_left_mouse_up(&mut self) -> Option<OnClick> {
        self.is_clicked = false;
        None
    }

    fn on_mouse_move(&mut self, p: &mut Take<Vec2>) -> Option<OnClick> {
        let bounds = self.dynamic_total_bounds();
        if let Some(pos) = p.peek() {
            if self.is_clicked {
                if let Some(off) = self.mouse_offset {
                    self.set_origin(*pos - off);
                }
            } else if self.is_hovered && bounds.contains(*pos) {
                self.mouse_offset = Some(*pos - self.origin);
            } else {
                self.mouse_offset = None;
            }
            self.is_hovered = bounds.contains(*pos);
            if self.is_hovered {
                p.take();
            }
        } else {
            self.is_hovered = false;
        }

        None
    }

    fn on_key(&mut self, key: &KeyboardInput) -> Option<OnClick> {
        if !self.is_focused {
            return None;
        }

        if !key.state.is_pressed() {
            return None;
        }

        match &key.logical_key {
            Key::Character(c) => {
                self.contents += c;
            }
            Key::Enter => {
                self.contents += "\n";
            }
            Key::Backspace => {
                self.contents.pop();
            }
            _ => (),
        }

        None
    }

    fn step(&mut self) -> Option<OnClick> {
        self.collapse_lpf.target = !self.is_minimized as u8 as f32;
        self.collapse_lpf.step();
        self.focus_animation.target = self.is_focused as u8 as f32;
        self.focus_animation.step();
        self.hover_animation.target = self.is_hovered as u8 as f32;
        self.hover_animation.step();
        self.origin += (self.target_origin - self.origin) * 0.3;
        None
    }
}

pub fn draw_window(canvas: &mut Canvas, window: &UiWindow, n: u32) {
    let shadow_alpha = window.focus_animation.actual * 0.45 + 0.5;
    let shadow_size = window.focus_animation.actual * 15.0 + 15.0;

    let alpha = 0.6 + 0.36 * window.collapse_lpf.actual.max(window.hover_animation.actual);

    canvas.rect(
        window.dynamic_total_bounds().padded(shadow_size),
        ZOrdering::Window(n, 1),
        BLACK.with_alpha(shadow_alpha),
    );

    let content = window.dynamic_content_bounds();
    let handle = window.handle_bounds();

    let factor = lerp(0.7, 0.4, window.focus_animation.actual);

    let orange = Srgba::from_f32_array([0.6, 0.3, 0.0, 0.8]);

    canvas.rect(
        content,
        ZOrdering::Window(n, 2),
        GRAY.with_alpha(alpha).mix(&BLACK, factor),
    );

    let handle_color = orange.with_alpha(alpha);

    canvas.rect(handle, ZOrdering::Window(n, 3), handle_color);

    for (b, _, color) in window.buttons() {
        let color = color.mix(&BLACK, 0.3);
        let b = b.offset(window.origin);
        canvas.rect(b, ZOrdering::Window(n, 4), color.with_alpha(alpha));
    }

    canvas
        .text(window.title.clone(), handle.mid_left() + Vec2::X * 5.0, 0.8)
        .set_anchor(Anchor::CenterLeft)
        .set_z_order(ZOrdering::Window(n, 5));

    canvas
        .text(
            window.contents.clone(),
            // format!("{:#?}", window),
            content.top_left() + Vec2::new(5.0, -5.0),
            0.7 * window.collapse_lpf.actual,
        )
        .set_anchor(Anchor::TopLeft)
        .set_z_order(ZOrdering::Window(n, 6));

    if let Some(p) = window.mouse_offset {
        let z = ZOrdering::Window(n, 7).as_f32();
        canvas.circle((window.origin + p).extend(z), 4.0, WHITE.with_alpha(0.2));
    }
}
