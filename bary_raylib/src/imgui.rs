use crate::assets::Assets;
use crate::client::ClientSpecificInfo;
use crate::render::draw::*;
use crate::sim::{Computer, VehicleGrid, World};
use crate::sounds::*;
use crate::ui::{Window, draw_window};
use bary_core::prelude::*;
use early_returns::*;
use raylib::prelude::*;

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

    let mut y = bottom_left.y;

    for (proto_id, proto) in world.prototypes.iter().rev() {
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
            Color::YELLOW
        } else {
            Color::DARKSLATEGRAY
        };

        let aabb = AABB::from_wh(box_width as f32, box_height as f32).with_center(center);

        let xc = bottom_left.x as f32 + box_width as f32 / 2.0;
        let yc = y as f32 + box_height as f32 / 4.0;
        let center = Vector2::new(xc, yc);
        d.draw_rectangle(bottom_left.x, y, box_width, box_height, color.alpha(alpha));
        if aabb.contains(mouse_pos) {
            d.draw_rectangle_lines(bottom_left.x, y, box_width, box_height, Color::WHITE);
            if editor.prototype_id != Some(*proto_id) {
                editor.prototype_id = Some(*proto_id);
                sounds.push(SoundEffect::PickLayer);
            }
        }
        draw_text_centered(d, &proto.name, center, font_size);
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

    for (layer, color) in boxes {
        let xc = origin.x as f32 + box_width as f32 / 2.0;
        let yc = y as f32 + box_height as f32 / 2.0;
        let center = Vec2::new(xc, yc);
        let aabb = AABB::from_wh(box_width as f32, box_height as f32).with_center(center);

        let xc = origin.x as f32 + box_width as f32 / 2.0;
        let yc = y as f32 + box_height as f32 / 4.0;
        let center = Vector2::new(xc, yc);
        let text = format!("{:?}", layer);
        let is_focused = Some(layer) == editor.layer || editor.layer.is_none();
        let alpha = if is_focused { 1.0 } else { 0.2 };
        d.draw_rectangle(origin.x, y, box_width, box_height, color.alpha(alpha));
        draw_text_centered(d, &text, center, font_size);

        if aabb.contains(mouse_pos) {
            d.draw_rectangle_lines(bottom_left.x, y, box_width, box_height, Color::WHITE);
            if editor.layer != Some(layer) {
                editor.layer = Some(layer);
                sounds.push(SoundEffect::PickLayer);
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

fn imgui_selected_grid_primary_computer_info(
    d: &mut RaylibDrawHandle,
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

    let window = Window {
        origin: IVec2::new(800, 60),
        title: "Grid Info".to_string(),
        content,
        is_focused: true,
    };

    if let Some(font) = &assets.fira_code {
        draw_window(d, &window, font);
    }
}

fn imgui_hovered_part_info(
    d: &mut RaylibDrawHandle,
    world: &World,
    client: &ClientSpecificInfo,
    assets: &Assets,
) {
    let mouse_pos = some_or_return!(client.mouse_screen_position);
    let gridloc = some_or_return!(client.hovered_grid_loc());
    let grid = ok_or_return!(world.grids.try_get(gridloc.grid_id));
    let occ = some_or_return!(grid.get_parts_at(gridloc.coord));

    let mut s = format!(
        "At {}-{}: {:?}",
        gridloc.grid_id,
        gridloc.coord,
        occ.to_array()
    );

    for (layer, part_id) in occ.iter() {
        let Ok(part) = world.parts.try_get(part_id) else {
            return;
        };

        s += &format!("\n\nPart ID: {}", part_id);
        s += &format!(
            "\nPlacement: {:?} {} {:?}",
            layer,
            part.placement.bottom_left(),
            part.placement.rot()
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
    }

    let window = Window {
        origin: mouse_pos.as_ivec2(),
        title: "Part Info".to_string(),
        content: s,
        is_focused: true,
    };

    if let Some(font) = &assets.fira_code {
        draw_window(d, &window, font);
    }
}

pub fn imgui_entrypoint(
    d: &mut RaylibDrawHandle,
    world: &mut World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
    assets: &Assets,
) {
    imgui_editor_layer_indicator(d, client, sounds);
    imgui_all_parts_in_layer(d, client, world, sounds);
    imgui_selected_grid_primary_computer_info(d, world, client, assets);
    imgui_hovered_part_info(d, world, client, assets);
}
