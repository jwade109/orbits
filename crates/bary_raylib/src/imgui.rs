use crate::app::App;
use crate::assets::Assets;
use crate::camera::to_raylib_camera;
use crate::client::ClientSpecificInfo;
use crate::cmd::draw_command_prompt;
use crate::render::draw::*;
use crate::sim::*;
use crate::sounds::*;
use crate::ui::{Window, draw_window};
use crate::utils::*;
use bary_core::prelude::PI;
use bary_core::prelude::*;
use bary_factory::*;
use bary_input::*;
use bary_parts::*;
use bary_sim::*;
use early_returns::*;
use enum_iterator::Sequence;
use log::*;
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
    pub fn button(&mut self, text: impl Into<String>) -> ButtonResponse {
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

        ButtonResponse {
            is_just_pressed,
            is_just_released,
            is_hot,
        }
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

pub struct ButtonResponse {
    is_just_pressed: bool,
    is_just_released: bool,
    is_hot: bool,
}

impl ButtonResponse {
    fn clicked(&self) -> bool {
        self.is_just_released
    }

    fn hovered(&self) -> bool {
        self.is_hot
    }

    fn just_released(&self) -> bool {
        self.is_just_pressed
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

pub fn imgui_test(
    gui: &mut ImGui,
    client: &mut ClientSpecificInfo,
    world: &mut World,
    sounds: &mut SoundEffects,
) {
    let mut layout = gui.layout(IVec2::splat(300), LayoutDirection::Down);

    let load = layout.button("Load");

    if load.just_released() {
        client.chat.log("Pressed!");
    }

    if load.clicked() {
        client.chat.log("Clicked!");
    }

    if layout.button("New Save").hovered() {
        client.chat.log("Hovered!");
    }

    layout.button("Settings");
    if layout.button("Exit Editor").clicked() {
        leave_ship_editor_on_escape(client, sounds);
    }

    if layout.button("Spawn Ship").clicked() {
        spawn_random_ship_on_p(world);
    }

    for portal in world.debug_portals.values_mut() {
        if let PortalState::Source(item) = &mut portal.state {
            let s = format!("{:?}", item);
            if layout.button(s).clicked() {
                *item = item.next().flatten();
            }
        }
    }

    drop(layout);

    let mut l2 = gui.layout(IVec2::new(800, 400), LayoutDirection::Right);

    l2.button("Hello!");
    l2.button("Goodbye!");

    drop(l2);

    ship_following_ui(gui, client, world);
}

fn ship_following_ui(gui: &mut ImGui, client: &ClientSpecificInfo, world: &World) {
    let grid_id = some_or_return!(client.focused_grid_id());
    let pos = some_or_return!(grid_pose(&world.grids, grid_id));

    let pos = get_world_to_screen(&client.camera, pos.translation, client.screen_dims);

    let mut ui = gui.layout(pos.as_ivec2() + IVec2::X * 20, LayoutDirection::Down);
    ui.button("Hello!");
    ui.button("Goodbye!");
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

fn imgui_selected_grid_primary_computer_info(
    _d: &mut RaylibDrawHandle,
    world: &World,
    client: &ClientSpecificInfo,
    assets: &Assets,
) {
    let free = some_or_return!(client.viewport.free());
    let grid_id = some_or_return!(free.selection_info.first_selected_grid());
    let grid = ok_or_return!(world.grids.try_get(grid_id));

    let mut content = grid_info_str(grid);

    if let Some(cpu_id) = grid.computers.first() {
        if let Ok(cpu) = world.computers.try_get(*cpu_id) {
            let info = computer_info_str(cpu);
            content += &format!("\n{}", info);
        }
    };

    let title = format!("Grid Info: \"{}\"", grid.name);

    let _window = Window {
        origin: IVec2::new(800, 60),
        title,
        content,
        is_focused: true,
    };

    if let Some(_font) = &assets.lato_regular {
        // draw_window(d, &window, font);
    }
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

fn draw_grid_far_indicators(
    grids: &Components<VehicleGrid>,
    d: &mut RaylibDrawHandle,
    client: &ClientSpecificInfo,
    camera: &Camera2D,
    assets: &Assets,
) {
    let free = some_or_return!(client.viewport.free());

    if camera.zoom > ZOOM_NEAR_FAR_THRESHOLD {
        return;
    }

    let marker_radius = 8.0f32;

    let mut markers = Vec::new();

    for (id, grid) in grids.iter() {
        let loc = grid.centroid_isometry();
        let p = glam_to_raylib_swap_y(loc.translation);
        let q = d.get_world_to_screen2D(p, camera);

        markers.push((
            *id,
            q,
            q,
            loc.rotation - camera.rotation.to_radians(),
            grid.name.clone(),
            !grid.computers.is_empty(),
        ));
    }

    // move the markers apart
    for _ in 0..10 {
        for i in 0..markers.len() {
            for j in 0..markers.len() {
                if i <= j {
                    continue;
                }

                let p1 = markers[i].1;
                let p2 = markers[j].1;
                let delta = p2 - p1;
                let dist = delta.length();
                if dist < marker_radius * 2.0 {
                    let u = delta.normalized();
                    let delta = marker_radius * 2.0 - dist;
                    markers[j].1 += u * delta / 2.0;
                    markers[i].1 -= u * delta / 2.0;
                }
            }
        }
    }

    let get_triangle = |center: Vector2, angle: f32| {
        let o = raylib_to_glam_invert_y(center);
        let u = Vec2::X * marker_radius;
        let a = o + rotate(u, angle);
        let b = o + rotate(u, angle + PI * 0.75);
        let c = o + rotate(u, angle - PI * 0.75);

        (
            glam_to_raylib_swap_y(a),
            glam_to_raylib_swap_y(b),
            glam_to_raylib_swap_y(c),
        )
    };

    let font = &assets.lato_regular;

    // draw the markers
    for (id, p, q, angle, name, is_controllable) in markers {
        let color = if is_controllable {
            Color::ORANGE
        } else {
            Color::GRAY
        };
        d.draw_line_v(p, q, color);
        if is_controllable {
            let (v1, v2, v3) = get_triangle(q, angle);
            d.draw_triangle(v1, v2, v3, color);
        }

        let is_hovered = Some(id) == free.selection_info.hovered.map(|g| g.grid_id);

        if !name.is_empty() {
            let color = if is_hovered {
                Color::WHITE
            } else if is_controllable && client.input.is_key_pressed(rdev::Key::ShiftLeft) {
                Color::WHITE.alpha(0.4)
            } else {
                Color::WHEAT.alpha(0.0)
            };
            let q = q - Vector2::new(0.0, 35.0);
            if let Some(font) = font {
                draw_text_centered(d, &font, &name, q, 30, color);
            } else {
                draw_text_centered_weak(d, &d.get_font_default(), &name, q, 30, color);
            }
        }
    }
}

pub fn lame_old_imgui_entrypoint(
    d: &mut RaylibDrawHandle,
    app: &mut App,
    sounds: &mut SoundEffects,
    assets: &Assets,
) {
    let raylib_camera = to_raylib_camera(&app.client.camera, app.client.screen_dims);

    draw_grid_far_indicators(&app.world.grids, d, &app.client, &raylib_camera, assets);

    imgui_editor_layer_indicator(d, &mut app.client, sounds);
    imgui_all_parts_in_layer(d, &mut app.client, &mut app.world, sounds);

    imgui_selected_grid_primary_computer_info(d, &mut app.world, &mut app.client, assets);
    imgui_hovered_part_info(d, &mut app.world, &mut app.client, assets);

    draw_command_prompt(d, &app.cmd, &assets);
}

fn selected_part_gui(gui: &mut ImGui, client: &ClientSpecificInfo, world: &mut World) {
    let loc = some_or_return!(client.selected_grid_loc());
    let grid = ok_or_return!(world.grids.try_get(loc.grid_id));
    let part_iso = ok_or_return!(gridloc_pose(&world.grids, loc));
    let pos = get_world_to_screen(&client.camera, part_iso.translation, client.screen_dims);
    let mut layout = gui.layout(vround(pos), LayoutDirection::Down);
    let occ = grid.get_parts_at(loc.coord);

    let s = distance_str_v(part_iso.translation.into());

    layout.button(s);
    layout.button("Follow");
    layout.button("Set Item");

    let occ = occ.unwrap_or(&PartOccupancy::EMPTY);

    if let Some(part_id) = occ.at_layer(PartLayer::Internal) {
        if let Ok(machine) = world.machines.try_get_mut(part_id) {
            selectable_ui(&mut layout, &mut machine.recipe);
        }
    }

    if let Some(part_id) = occ.at_layer(PartLayer::Plumbing) {
        if let Ok(portal) = world.debug_portals.try_get_mut(part_id) {
            if let PortalState::Source(item) = &mut portal.state {
                selectable_ui(&mut layout, item);
            }
        }
    }
}

fn save_editor_vehicle_as_blueprint(client: &mut ClientSpecificInfo, world: &World) {
    client.chat.log("Save blueprint");
    let editor = some_or_return!(client.viewport.editor());

    let bp = get_blueprint(world, editor.vehicle);

    if let Ok(bp) = bp {
        let bytes = bincode::serialize(&bp).unwrap();
        let digest = md5::compute(bytes);

        let path = format!("/tmp/test.bp");

        if let Err(e) = save_blueprint(&path, &bp) {
            error!("Failed to save blueprint: {e:?}");
        } else {
            client.chat.log(format!("Saved to {}", path));
        }
    }
}

fn editor_gui(
    gui: &mut ImGui,
    world: &World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
) {
    let editor = some_or_return!(client.viewport.editor());

    let mut layout = gui.layout(IVec2::new(100, 20), LayoutDirection::Right);

    layout.button(format!("Editing {}", editor.vehicle));

    if layout.button("Save").clicked() {
        save_editor_vehicle_as_blueprint(client, world);
    };
    if layout.button("Exit Editor").clicked() {
        leave_ship_editor_on_escape(client, sounds);
    }
}

fn free_gui(
    gui: &mut ImGui,
    world: &mut World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
) {
    let _free = some_or_return!(client.viewport.free());

    let mut layout = gui.layout(IVec2::new(100, 20), LayoutDirection::Right);

    if let Some(id) = client.focused_grid_id() {
        if layout.button(format!("Edit Grid {}", id)).clicked() {
            enter_ship_editor(world, client, sounds);
        }
    }

    if layout.button("<<").clicked() {
        if world.tick_rate > 1 {
            world.tick_rate -= 1;
        }
    }
    if layout.button(">>").clicked() {
        world.tick_rate += 1;
    }
}

pub fn selectable_ui<T>(layout: &mut LayoutHandle, value: &mut T)
where
    T: Sequence + std::fmt::Debug,
{
    if let Some(p) = value.previous() {
        if layout.button("Previous").clicked() {
            *value = p;
        }
    }
    let s = format!("{:?}", value);
    layout.button(s);
    if let Some(n) = value.next() {
        if layout.button("Next").clicked() {
            *value = n;
        }
    }
}

pub fn imgui_pass(
    client: &mut ClientSpecificInfo,
    world: &mut World,
    sounds: &mut SoundEffects,
) -> ImGui {
    let mut gui = ImGui::new(
        client.screen_dims,
        client.mouse_screen_position,
        client.input.clone(),
    );

    selected_part_gui(&mut gui, client, world);

    free_gui(&mut gui, world, client, sounds);
    editor_gui(&mut gui, world, client, sounds);

    gui
}
