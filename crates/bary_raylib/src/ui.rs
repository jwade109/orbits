use crate::assets::*;
use bary_core::prelude::*;
use raylib::prelude::*;

pub fn draw_window(d: &mut RaylibDrawHandle, window: &Window, font: &Font) {
    let font_size = 18;
    let spacing = 1.0;
    let padding = 3;
    let child_gap = 0;
    let shadow_width = 3;

    let title_dims = font.measure_text(&window.title, font_size as f32, spacing);
    let title_dims = IVec2::new(title_dims.x as i32, title_dims.y as i32);
    let header_height: i32 = title_dims.y + padding * 2;

    let title_text_origin = window.origin + IVec2::splat(padding);

    let text_dims = font.measure_text(&window.content, font_size as f32, spacing);
    let text_dims = IVec2::new(text_dims.x as i32, text_dims.y as i32);
    let content_dims = text_dims + IVec2::splat(2 * padding);
    let window_dims =
        content_dims + IVec2::new(2 * padding, child_gap + 2 * padding + header_height);

    let content_origin = window.origin + IVec2::new(padding, padding + header_height + child_gap);
    let text_origin = content_origin + IVec2::splat(padding);

    let shadow_origin = window.origin - IVec2::splat(shadow_width);
    let shadow_dims = window_dims + IVec2::splat(shadow_width * 2);

    let alpha = if window.is_focused { 1.0 } else { 0.2 };

    let gray = Color::new(20, 20, 20, 255);

    // shadow
    d.draw_rectangle(
        shadow_origin.x,
        shadow_origin.y,
        shadow_dims.x,
        shadow_dims.y,
        Color::BLACK.alpha(0.7),
    );

    // whole window
    d.draw_rectangle(
        window.origin.x,
        window.origin.y,
        window_dims.x,
        window_dims.y,
        gray.alpha(alpha),
    );

    // window header
    d.draw_rectangle(
        window.origin.x,
        window.origin.y,
        window_dims.x,
        header_height,
        gray.alpha(alpha),
    );

    // content area
    d.draw_rectangle(
        content_origin.x,
        content_origin.y,
        content_dims.x,
        content_dims.y,
        Color::new(40, 40, 40, 255).alpha(alpha),
    );

    let position = Vector2::new(text_origin.x as f32, text_origin.y as f32);
    // d.set_text_line_spacing(0);

    d.draw_text_ex(
        font,
        &window.title,
        Vector2::new(title_text_origin.x as f32, title_text_origin.y as f32),
        font_size as f32,
        spacing,
        Color::WHITE.alpha(alpha),
    );

    d.draw_text_ex(
        font,
        &window.content,
        position,
        font_size as f32,
        spacing,
        Color::WHITE.alpha(alpha),
    );
}

pub struct Window {
    pub origin: IVec2,
    pub title: String,
    pub content: String,
    pub is_focused: bool,
}

impl Window {
    pub fn new(
        origin: IVec2,
        title: impl Into<String>,
        content: impl Into<String>,
        is_focused: bool,
    ) -> Self {
        Self {
            origin,
            title: title.into(),
            content: content.into(),
            is_focused,
        }
    }
}

struct WindowLocation {
    origin: IVec2,
    dims: IVec2,
    is_focused: bool,
}

impl WindowLocation {
    fn to_rect(&self) -> Rectangle {
        let o = self.origin.as_vec2();
        let s = self.dims.as_vec2();
        Rectangle::new(o.x, o.y, s.x, s.y)
    }

    fn contains(&self, pos: Vec2) -> bool {
        let delta = pos - self.origin.as_vec2();
        let d = self.dims.as_vec2();
        delta.x >= 0.0 && delta.y >= 0.0 && delta.x <= d.x && delta.y <= d.y
    }

    fn contains_opt(&self, pos: Option<Vec2>) -> bool {
        pos.map(|p| self.contains(p)).unwrap_or(false)
    }
}

struct GridInfoWindow {
    grid_id: Ent,
}

struct PartInfoWindow {
    part_id: Ent,
}

fn draw_window_bounding_boxes(d: &mut RaylibDrawHandle, locs: &Components<WindowLocation>) {
    for loc in locs.values() {
        let rec = loc.to_rect();
        d.draw_rectangle_lines_ex(rec, 1.0, Color::RED);
    }
}

fn draw_windows(d: &mut RaylibDrawHandle, assets: &Assets, windows: &[Window]) {
    let Some(font) = &assets.fira_code else {
        return;
    };

    for window in windows {
        draw_window(d, &window, font);
    }
}
