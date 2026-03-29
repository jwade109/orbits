use crate::app::App;
use crate::assets::Assets;
use crate::camera::to_raylib_camera;
use crate::client::ClientSpecificInfo;
use crate::cmd::prompt::draw_command_prompt;
use crate::components::Components;
use crate::render::draw::*;
use crate::sim::{Computer, VehicleGrid, World};
use crate::sounds::*;
use crate::ui::{Window, draw_window};
use crate::utils::*;
use bary_core::prelude::PI;
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

    let font = d.get_font_default();

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

            if client.input.just_pressed_debounced(rdev::Key::KeyZ) {
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

    let title = format!("Grid Info: \"{}\"", grid.name);

    let window = Window {
        origin: IVec2::new(800, 60),
        title,
        content,
        is_focused: true,
    };

    if let Some(font) = &assets.fira_code {
        draw_window(d, &window, font);
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

    struct MarkerInfo {
        id: Ent,
        is_controllable: bool,
        is_hovered: bool,
        rotation: f32,
        name: String,
    }

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

    let font = &assets.fira_code;

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
            } else if is_controllable {
                Color::WHITE.alpha(0.3)
            } else {
                Color::WHEAT.alpha(0.02)
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

pub fn imgui_entrypoint(
    d: &mut RaylibDrawHandle,
    app: &mut App,
    sounds: &mut SoundEffects,
    assets: &Assets,
) {
    let raylib_camera = to_raylib_camera(
        &app.runner.client_info.camera,
        app.runner.client_info.screen_dims,
    );

    draw_grid_far_indicators(
        &app.runner.world.grids,
        d,
        &app.runner.client_info,
        &raylib_camera,
        assets,
    );

    imgui_editor_layer_indicator(d, &mut app.runner.client_info, sounds);
    imgui_all_parts_in_layer(
        d,
        &mut app.runner.client_info,
        &mut app.runner.world,
        sounds,
    );
    imgui_selected_grid_primary_computer_info(
        d,
        &mut app.runner.world,
        &mut app.runner.client_info,
        assets,
    );
    imgui_hovered_part_info(
        d,
        &mut app.runner.world,
        &mut app.runner.client_info,
        assets,
    );

    draw_command_prompt(d, &app.cmd, &assets);
}
