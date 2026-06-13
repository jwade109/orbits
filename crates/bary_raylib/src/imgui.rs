use crate::assets::Assets;
use crate::render::*;
use crate::sim::*;
use crate::sounds::*;
use crate::ui::{Window, draw_window};
use crate::utils::*;
use bary_core::prelude::*;
use bary_factory::*;
use bary_input::*;
use bary_sim::*;
use early_returns::*;
use raylib::prelude::*;

#[derive(Debug, Copy, Clone)]
pub enum LayoutDirection {
    Down,
    Right,
}

#[derive(Clone, Debug)]
pub struct TextArea {
    pub id: i64,
    pub origin: IVec2,
    pub dims: IVec2,
    pub text: String,
    pub is_hot: bool,
    pub is_pressed: bool,
    pub is_just_pressed: bool,
    pub is_just_released: bool,
}

pub struct ImGui {
    pub id_counter: i64,
    pub screen: Vec2,
    pub mouse_pos: Option<Vec2>,
    pub input: InputState,
    pub active: i64,
    pub layouts: Vec<Layout>,
}

pub struct Layout {
    pub id: i64,
    pub origin: IVec2,
    pub dims: IVec2,
    pub text_areas: Vec<TextArea>,
}

pub struct LayoutHandle<'a> {
    id: i64,
    text_areas: Vec<TextArea>,
    origin: IVec2,
    dir: LayoutDirection,
    dims: IVec2,
    padding: i32,
    child_gap: i32,
    next_pos: IVec2,
    gui: &'a mut ImGui,
}

fn ui_bounds(origin: IVec2, dims: IVec2) -> AABB {
    let br = origin + dims;
    AABB::from_arbitrary(origin.as_vec2(), br.as_vec2())
}

impl<'a> LayoutHandle<'a> {
    pub fn button(&mut self, text: impl Into<String>) {
        let id = self.gui.next_id_counter();
        let dims = IVec2::new(155, 40);
        let origin = self.next_pos;
        let aabb = ui_bounds(origin, dims);
        let is_hot = self
            .gui
            .mouse_pos
            .map(|p| aabb.contains(p))
            .unwrap_or(false);
        let is_pressed = is_hot && self.gui.input.is_key_pressed(rdev::Button::Left);
        let is_just_pressed = is_hot && self.gui.input.just_pressed_debounced(rdev::Button::Left);
        let is_just_released = is_hot && self.gui.input.just_released(rdev::Button::Left);

        match self.dir {
            LayoutDirection::Down => {
                self.dims.x = self.dims.x.max(dims.x + self.padding * 2);
                self.dims.y += dims.y;
                self.next_pos += IVec2::new(0, dims.y + self.child_gap);
            }
            LayoutDirection::Right => {
                self.dims.y = self.dims.y.max(dims.y + self.padding * 2);
                self.dims.x += dims.x;
                self.next_pos += IVec2::new(dims.x + self.child_gap, 0);
            }
        }

        if !self.text_areas.is_empty() {
            match self.dir {
                LayoutDirection::Down => self.dims.y += self.child_gap,
                LayoutDirection::Right => self.dims.x += self.child_gap,
            }
        }

        if is_hot {
            self.gui.active = id;
        }

        let button = TextArea {
            id,
            origin,
            dims,
            text: text.into(),
            is_hot,
            is_pressed,
            is_just_pressed,
            is_just_released,
        };

        self.text_areas.push(button);
    }
}

impl<'a> std::ops::Drop for LayoutHandle<'a> {
    fn drop(&mut self) {
        let aabb = ui_bounds(self.origin, self.dims);
        let mp = self.gui.mouse_pos.unwrap_or(Vec2::NAN);
        let is_hot = aabb.contains(mp);

        if is_hot && self.gui.active < self.id {
            self.gui.active = self.id;
        }

        self.gui.layouts.push(Layout {
            id: self.id,
            origin: self.origin,
            dims: self.dims,
            text_areas: self.text_areas.clone(),
        });
    }
}

impl ImGui {
    pub fn new(screen: Vec2, mouse_pos: Option<Vec2>, input: InputState) -> Self {
        let is_on_screen = if let Some(mp) = mouse_pos {
            mp.x >= 0.0 && mp.y >= 0.0 && mp.x <= screen.x && mp.y <= screen.y
        } else {
            false
        };

        let active = if is_on_screen { 0 } else { -1 };

        Self {
            id_counter: 1,
            screen,
            mouse_pos,
            input,
            active,
            layouts: Vec::new(),
        }
    }

    fn next_id_counter(&mut self) -> i64 {
        let ret = self.id_counter;
        self.id_counter += 1;
        ret
    }

    pub fn layout<'a>(&'a mut self, pos: IVec2, dir: LayoutDirection) -> LayoutHandle<'a> {
        let padding = 7;
        LayoutHandle {
            id: self.next_id_counter(),
            dir,
            padding,
            child_gap: 4,
            origin: pos,
            dims: IVec2::splat(padding * 2),
            next_pos: pos + IVec2::splat(padding),
            gui: self,
            text_areas: Vec::new(),
        }
    }

    pub fn active_id(&self) -> i64 {
        self.active
    }

    pub fn is_hovering_gui(&self) -> bool {
        self.active > 0
    }
}

pub fn imgui_all_parts_in_layer(
    d: &mut RaylibDrawHandle,
    client: &mut ClientSpecificInfo,
    world: &World,
    sounds: &mut SoundEffects,
) {
    let hovered_proto = get_hovered_prototype(client, world);
    let editor = some_or_return!(client.viewport.editor_mut());
    let layer = some_or_return!(editor.layer);
    let mouse_pos = client.mouse_screen_position.unwrap_or(Vec2::NAN);

    let height = d.get_render_height();

    let bottom_left = IVec2::new(50, height - 50);
    let font_size = 18;
    let box_width = 200;
    let box_height = 40;
    let padding = 2;

    let font = d.get_font_default();

    let mut y = bottom_left.y;

    for (proto_id, proto) in world.prototypes.iter() {
        if proto.layer != layer {
            continue;
        }
        let is_hovered = Some(*proto_id) == hovered_proto;
        let is_selected = Some(*proto_id) == editor.prototype_id;
        y -= box_height + padding;

        let xc = bottom_left.x as f32 + box_width as f32 / 2.0;
        let yc = y as f32 + box_height as f32 / 2.0;
        let center = Vec2::new(xc, yc);

        let alpha = 0.96;

        let color = if is_selected {
            Color::ORANGE
        } else if is_hovered {
            Color::TEAL
        } else {
            Color::DARKSLATEGRAY
        };

        let aabb = AABB::from_wh(box_width as f32, box_height as f32).with_center(center);

        d.draw_rectangle(bottom_left.x, y, box_width, box_height, color.alpha(alpha));
        if aabb.contains(mouse_pos) {
            d.draw_rectangle_lines(bottom_left.x, y, box_width, box_height, Color::WHITE);
            if editor.prototype_id != Some(*proto_id) {
                editor.prototype_id = Some(*proto_id);
                sounds.push(SoundEffect::PickLayer);
            }
        }
        draw_text_centered_weak(
            d,
            &font,
            &proto.name,
            glam_to_raylib(center),
            font_size,
            Color::WHITE,
        );
    }
}

pub fn imgui_editor_layer_indicator(
    d: &mut RaylibDrawHandle,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
) {
    let editor = some_or_return!(client.viewport.editor_mut());
    let mouse_pos = client.mouse_screen_position.unwrap_or(Vec2::NAN);

    let boxes = [
        (PartLayer::Exterior, Color::WHITE),
        (PartLayer::Structural, Color::GRAY),
        (PartLayer::Plumbing, Color::PURPLE),
        (PartLayer::Internal, Color::ORANGE),
    ];

    let width = d.get_render_width();
    let height = d.get_render_height();

    let font_size = 18;
    let box_width = 200;
    let box_height = 40;
    let padding = 2;
    let bottom_right = IVec2::new(width - 50, height - 50);
    let dims = IVec2::new(
        box_width,
        boxes.len() as i32 * box_height + padding * (boxes.len() as i32 - 1),
    );
    let origin = bottom_right - dims;
    let bottom_left = bottom_right - IVec2::X * dims.x;

    let mut y = origin.y;

    let font = d.get_font_default();

    for (layer, color) in boxes {
        let xc = origin.x as f32 + box_width as f32 / 2.0;
        let yc = y as f32 + box_height as f32 / 2.0;
        let center = Vec2::new(xc, yc);
        let aabb = AABB::from_wh(box_width as f32, box_height as f32).with_center(center);

        let text = format!("{:?}", layer);
        let is_focused = Some(layer) == editor.layer || editor.layer.is_none();
        let alpha = if is_focused { 1.0 } else { 0.2 };
        d.draw_rectangle(origin.x, y, box_width, box_height, color.alpha(alpha));

        draw_text_centered_weak(
            d,
            &font,
            &text,
            glam_to_raylib(center),
            font_size,
            Color::WHITE,
        );

        if aabb.contains(mouse_pos) {
            d.draw_rectangle_lines(bottom_left.x, y, box_width, box_height, Color::WHITE);

            if client.input.just_pressed_debounced(rdev::Button::Left) {
                if editor.layer != Some(layer) {
                    editor.layer = Some(layer);
                    sounds.push(SoundEffect::PickLayer);
                }
            }
        }

        y += box_height + padding;
    }
}

fn grid_info_str(grid: &VehicleGrid) -> String {
    let lines = [
        format!("GRID INFO ==="),
        format!("\n  Parts: {}", grid.parts.len()),
        format!("\n  Thrusters: {}", grid.thrusters.len()),
        format!("\n  Computers: {}", grid.computers.len()),
        format!("\n  Parts mass: {}", grid.parts_mass),
    ];

    lines.into_iter().collect()
}

fn computer_info_str(cpu: &Computer) -> String {
    let mut lines = vec![
        format!("CPU INFO ==="),
        format!("\n  On: {}", cpu.on),
        format!("\n  Status: {:?}", cpu.status),
        format!("\n  Ticks: {}", cpu.ticks_this_cycle),
        format!("\n  Fired: {}", cpu.fired_this_tick),
        format!("\n  Iters: {}", cpu.iters),
    ];

    for cmd in &cpu.command_queue {
        let line = format!("\n  - {}", cmd);
        lines.push(line);
    }

    lines.into_iter().collect()
}

fn slot_info_str(slot: &InvSlot) -> String {
    if let Some(contents) = slot.contents() {
        format!(
            "\n  - {:?} ({:0.1}%) {} {} {}",
            contents,
            100.0 * slot.fill_percentage(),
            slot.mass(),
            slot.location().0,
            slot.location().1,
        )
    } else {
        format!("\n  - Empty - {:?}", slot.filter())
    }
}

fn inventory_info_str(inv: &Inventory) -> String {
    let mut lines = vec![format!("INVENTORY")];

    for slot in inv.slots() {
        let line = slot_info_str(slot);
        lines.push(line);
    }

    lines.into_iter().collect()
}

pub const ZOOM_NEAR_FAR_THRESHOLD: f32 = 5.0;
pub const ZOOM_NEAR_VEHICLE: f32 = 60.0;
pub const ZOOM_FAR_AWAY: f32 = 1.0;

fn imgui_hovered_part_info(
    d: &mut RaylibDrawHandle,
    world: &World,
    client: &ClientSpecificInfo,
    assets: &Assets,
) {
    if client.camera.zoom < ZOOM_NEAR_FAR_THRESHOLD {
        return;
    }

    let gridloc = some_or_return!(client.hovered_grid_loc());
    let grid = ok_or_return!(world.grids.try_get(gridloc.grid_id));
    let occ = some_or_return!(grid.get_parts_at(gridloc.coord));

    let mut s = format!(
        "At {}-{}: {:?}",
        gridloc.grid_id,
        gridloc.coord,
        occ.to_array()
    );

    let slot = get_slot_c(gridloc, &world.grids, &world.parts, &world.inventories);
    if let Ok(slot) = slot {
        s += &format!("\n\nInventory slot here: {}", slot_info_str(slot));
    }

    for (layer, part_id) in occ.iter() {
        let part = ok_or_continue!(world.parts.try_get(part_id));
        let part_local = part.region.to_local(gridloc.coord);

        s += &format!("\n\nPart ID: {}", part_id);
        s += &format!("\nPart local coord: {}", part_local);

        s += &format!(
            "\nRegion: {:?} {} {:?}",
            layer,
            part.region.bottom_left(),
            part.region.rot()
        );

        if let Ok(proto) = world.prototypes.try_get(part.prototype) {
            s += &format!(
                "\nPrototype: {} {} {:?}",
                proto.name,
                proto.mass,
                proto.classification()
            );
        }
        if let Ok(cpu) = world.computers.try_get(part_id) {
            let info = computer_info_str(cpu);
            s += &format!("\n{}", info);
        }
        if let Ok(thruster) = world.thrusters.try_get(part_id) {
            s += &format!("\n{:#?}", thruster);
        }
        if let Ok(light) = world.lights.try_get(part_id) {
            s += &format!("\n{:#?}", light);
        }
        if let Ok(mac) = world.machines.try_get(part_id) {
            s += &format!("\n{:#?}", mac);
        }
        if let Ok(inv) = world.inventories.try_get(part_id) {
            let info = inventory_info_str(inv);
            s += &format!("\n{}", info);
        }
    }

    let origin = vround(Vec2::new(client.screen_dims.x - 500.0, 20.0));

    let window = Window {
        origin,
        title: "Part Info".to_string(),
        content: s,
        is_focused: true,
    };

    if let Some(font) = &assets.lato_regular {
        draw_window(d, &window, font);
    }
}

pub fn lame_old_imgui_entrypoint(
    d: &mut RaylibDrawHandle,
    client: &mut ClientSpecificInfo,
    world: &World,
    sounds: &mut SoundEffects,
    assets: &Assets,
) {
    imgui_editor_layer_indicator(d, client, sounds);
    imgui_all_parts_in_layer(d, client, world, sounds);
    imgui_hovered_part_info(d, world, client, assets);
}
