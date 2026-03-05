use crate::{
    components::{Components, EntitySpawner},
    world::{Assets, World},
};
use bary_core::prelude::*;
use raylib::prelude::*;

pub fn draw_window(d: &mut RaylibDrawHandle, window: &Window, font: &Font) {
    let font_size = 23;
    let spacing = 1.0;
    let padding = 7;
    let child_gap = 0;
    let shadow_width = if window.is_focused { 11 } else { 3 };

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
        Color::GRAY.alpha(alpha),
    );

    // window header
    d.draw_rectangle(
        window.origin.x,
        window.origin.y,
        window_dims.x,
        header_height,
        Color::ORANGE.alpha(alpha),
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
    d.set_text_line_spacing(0);

    d.draw_text_ex(
        font,
        &window.title,
        Vector2::new(title_text_origin.x as f32, title_text_origin.y as f32),
        font_size as f32,
        spacing,
        Color::BLACK.alpha(alpha),
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

pub struct UiState {
    spawner: EntitySpawner,
    grid_info: Components<GridInfoWindow>,
    part_info: Components<PartInfoWindow>,
    location: Components<WindowLocation>,
}

pub fn update_ui_state(ui: &mut UiState, mouse_screen_position: Option<Vec2>) {
    for loc in ui.location.values_mut() {
        loc.is_focused = loc.contains_opt(mouse_screen_position);
    }
}

fn draw_window_bounding_boxes(d: &mut RaylibDrawHandle, locs: &Components<WindowLocation>) {
    for loc in locs.values() {
        let rec = loc.to_rect();
        d.draw_rectangle_lines_ex(rec, 1.0, Color::RED);
    }
}

pub fn compile_windows(ui: &UiState, world: &World) -> Vec<Window> {
    let mut ret = Vec::new();

    for (id, grid_info) in ui.grid_info.iter() {
        let Ok(loc) = ui.location.try_get(*id) else {
            continue;
        };
        let Ok(grid) = world.grids.try_get(grid_info.grid_id) else {
            continue;
        };

        let content = format!("{:#?}", grid.body_frame_forces);
        let title = format!("Grid {} Info", grid_info.grid_id);
        let window = Window::new(loc.origin, title, content, loc.is_focused);
        ret.push(window);
    }

    for (id, part_info) in ui.part_info.iter() {
        let Ok(loc) = ui.location.try_get(*id) else {
            continue;
        };
        let Ok(part) = world.parts.try_get(part_info.part_id) else {
            continue;
        };

        let content = format!("{:#?}", part);
        let title = format!("Part {} Info", part_info.part_id);
        let window = Window::new(loc.origin, title, content, loc.is_focused);
        ret.push(window);
    }

    ret
}

pub fn draw_ui(d: &mut RaylibDrawHandle, world: &World, ui: &UiState, assets: &Assets) {
    let windows = compile_windows(ui, world);
    draw_windows(d, assets, &windows);
    draw_window_bounding_boxes(d, &ui.location);
}

fn draw_windows(d: &mut RaylibDrawHandle, assets: &Assets, windows: &[Window]) {
    let Some(font) = &assets.fira_code else {
        return;
    };

    for window in windows {
        draw_window(d, &window, font);
    }
}

impl UiState {
    pub fn new() -> Self {
        Self {
            spawner: EntitySpawner::default(),
            grid_info: Components::default(),
            part_info: Components::default(),
            location: Components::default(),
        }
    }

    pub fn track_grid_info(&mut self, grid_id: Ent) {
        let id = self.spawner.spawn();
        self.location.spawn(
            id,
            WindowLocation {
                origin: IVec2::splat(200),
                dims: IVec2::new(500, 700),
                is_focused: true,
            },
        );
        self.grid_info.spawn(id, GridInfoWindow { grid_id });
    }

    pub fn track_part_info(&mut self, part_id: Ent) {
        let id = self.spawner.spawn();
        self.location.spawn(
            id,
            WindowLocation {
                origin: IVec2::new(600, 170),
                dims: IVec2::new(500, 700),
                is_focused: true,
            },
        );
        self.part_info.spawn(id, PartInfoWindow { part_id });
    }
}
