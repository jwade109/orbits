use crate::prelude::*;
use starling::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum WindowClass {
    Tutorial,
    Hello,
    VehicleInfo,
}

impl WindowClass {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Tutorial => "Tutorial",
            Self::Hello => "Hello!",
            Self::VehicleInfo => "Vehicle Info",
        }
    }
}

#[derive(Debug)]
pub struct UiWindow {
    pub class: WindowClass,
    pub title: String,
    pub contents: String,
    pub origin: Vec2,
    pub static_content_dims: Vec2,
    pub handle_height: f32,
    pub is_hovered: bool,
    pub is_clicked: bool,
    pub mouse_offset: Option<Vec2>,
    is_focused: bool,
    is_minimized: bool,
    collapse_lpf: Lpf,
}

impl UiWindow {
    pub fn new(class: WindowClass, contents: impl Into<String>) -> Self {
        Self {
            class,
            title: class.title().into(),
            contents: contents.into(),
            origin: randvec(100.0, 400.0),
            static_content_dims: Vec2::new(450.0, rand(200.0, 500.0)),
            handle_height: 30.0,
            is_hovered: false,
            is_clicked: false,
            mouse_offset: None,
            is_focused: false,
            is_minimized: false,
            collapse_lpf: Lpf::new(1.0, 0.0, 0.3),
        }
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

    pub fn static_minimize_button(&self) -> AABB {
        let cb = self.handle_bounds();
        let tr = cb.upper();
        let h = cb.span.y;
        let bl = tr - Vec2::splat(h);
        AABB::from_arbitrary(bl, tr).padded(-10.0)
    }

    pub fn dynamic_minimize_button(&self) -> AABB {
        let b = self.static_minimize_button();
        b.padded(if self.is_min_button_clicked() {
            -5.0
        } else {
            0.0
        })
    }

    pub fn is_min_button_clicked(&self) -> bool {
        if let Some(off) = self.mouse_offset {
            self.is_clicked && self.static_minimize_button().contains(off + self.origin)
        } else {
            false
        }
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
}

impl Interactive for UiWindow {
    fn on_left_mouse_down(&mut self) -> Option<OnClick> {
        if self.is_hovered {
            self.is_clicked = true;
            self.is_focused = true;
        } else {
            self.is_focused = false;
        }
        None
    }

    fn on_left_mouse_up(&mut self) -> Option<OnClick> {
        self.is_clicked = false;
        if let Some(off) = self.mouse_offset {
            if self.static_minimize_button().contains(self.origin + off) {
                self.is_minimized = !self.is_minimized;
            }
        }
        None
    }

    fn on_mouse_move(&mut self, p: &mut Take<Vec2>) {
        let bounds = self.dynamic_total_bounds();
        if let Some(pos) = p.peek() {
            if self.is_clicked {
                if let Some(off) = self.mouse_offset {
                    self.origin = *pos - off;
                }
            } else {
                let handle_bounds = self.handle_bounds();
                if handle_bounds.contains(*pos) {
                    self.mouse_offset = Some(*pos - self.origin);
                } else {
                    self.mouse_offset = None;
                }
            }
            self.is_hovered = bounds.contains(*pos);
            if self.is_hovered {
                p.take();
            }
        } else {
            self.is_hovered = false;
        }
    }

    fn step(&mut self) {
        self.collapse_lpf.step();
        self.collapse_lpf.target = !self.is_minimized as u8 as f32;

        let desired_pos = vround(self.origin / 25.0).as_vec2() * 25.0;
        if !self.is_clicked || !self.mouse_offset.is_some() {
            self.origin += (desired_pos - self.origin) * 0.2;
        }
    }
}

pub fn draw_window(canvas: &mut Canvas, window: &UiWindow, n: u32) {
    let (shadow_alpha, shadow_size) = if window.is_focused {
        (0.95, 30.0)
    } else {
        (0.5, 15.0)
    };

    let alpha = 0.1 + 0.8 * window.collapse_lpf.actual;

    canvas.rect(
        window.dynamic_total_bounds().padded(shadow_size),
        ZOrdering::Window(n, 1),
        BLACK.with_alpha(shadow_alpha),
    );

    let content = window.dynamic_content_bounds();
    let handle = window.handle_bounds();
    let mb = window.dynamic_minimize_button();

    let factor = if window.is_focused { 0.3 } else { 0.7 };

    // let inner = content.padded(-pad);
    canvas.rect(
        content,
        ZOrdering::Window(n, 2),
        GRAY.with_alpha(alpha).mix(&BLACK, factor),
    );

    let handle_color = DARK_BLUE.mix(&BLACK, factor).with_alpha(alpha);

    canvas.rect(handle, ZOrdering::Window(n, 3), handle_color);

    let color = WHITE.mix(&BLACK, factor);

    canvas.rect(mb, ZOrdering::Window(n, 4), color.with_alpha(alpha));

    let tl = canvas.text(window.title.clone(), handle.mid_left() + Vec2::X * 5.0, 0.8);

    tl.anchor_left();
    tl.z_index = ZOrdering::Window(n, 5);

    let tl = canvas.text(
        window.contents.clone(),
        // format!("{:#?}", window),
        content.top_left() + Vec2::new(5.0, -5.0),
        0.7 * window.collapse_lpf.actual,
    );
    tl.z_index = ZOrdering::Window(n, 6);
    tl.color.alpha = alpha;
    tl.anchor_top_left();
}
