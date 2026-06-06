use crate::assets::*;
use crate::camera::{Camera, to_raylib_camera};
use crate::editor_state::EditorState;
use crate::imgui::{ImGui, ZOOM_NEAR_FAR_THRESHOLD};
use crate::sim::*;
use crate::utils::*;
use bary_core::prelude::PI;
use bary_core::prelude::*;
use bary_factory::*;
use bary_orbital::VehicleControl;
use bary_parts::*;
use bary_sim::*;
use bary_terminal::*;
use early_returns::*;
use raylib::prelude::*;

fn draw_text(d: &mut RaylibDrawHandle, iso: Isometry2d, text: &str, font_size: f32) {
    let p = glam_to_raylib_swap_y(iso.translation);
    if !text.is_empty() {
        d.draw_text_pro(
            d.get_font_default(),
            &text,
            p,
            Vector2::zero(),
            -iso.rotation.to_degrees(),
            font_size,
            font_size / 20.0,
            Color::ORANGE,
        );
    }
}

fn draw_text_centered_bg(d: &mut RaylibDrawHandle, iso: Isometry2d, text: &str, font_size: f32) {
    if text.is_empty() {
        return;
    }

    let spacing = font_size / 20.0;
    let dims = raylib_to_glam(d.get_font_default().measure_text(text, font_size, spacing));
    let iso = iso.offset(-dims / 2.0);

    fill_rectangle(d, iso, dims, Color::BLACK);

    let text_iso = iso.offset(Vec2::Y * font_size);
    let p = glam_to_raylib_swap_y(text_iso.translation);

    d.draw_text_pro(
        d.get_font_default(),
        &text,
        p,
        Vector2::zero(),
        -iso.rotation.to_degrees(),
        font_size,
        font_size / 20.0,
        Color::WHITE,
    );
}

fn draw_isometry_axes(d: &mut RaylibDrawHandle, iso: Isometry2d, label: &str, scale: Vec2) {
    let x = iso.translation + iso.local_x() * scale.x;
    let y = iso.translation + iso.local_y() * scale.y;

    let p = glam_to_raylib_swap_y(iso.translation);
    let x = glam_to_raylib_swap_y(x);
    let y = glam_to_raylib_swap_y(y);

    // d.draw_circle_v(p, 0.1, Color::WHITE);
    // d.draw_circle_v(x, 0.1, Color::RED);
    // d.draw_circle_v(y, 0.1, Color::GREEN);

    d.draw_line_ex(p, x, 0.1, Color::RED);
    d.draw_line_ex(p, y, 0.1, Color::GREEN);

    draw_text(d, iso, label, scale.max_element());
}

fn draw_origin_and_range_indicators(d: &mut RaylibDrawHandle) {
    let c = Color::GRAY;
    draw_line(d, -Vec2::X * 10000.0, Vec2::X * 10000.0, c.alpha(0.4));
    draw_line(d, -Vec2::Y * 10000.0, Vec2::Y * 10000.0, c.alpha(0.4));
    draw_line(d, Vec2::ZERO, Vec2::X * 10.0, c);
    draw_line(d, Vec2::ZERO, Vec2::Y * 10.0, c);
    for r in (1000..=10000).step_by(1000) {
        draw_circle(d, Vec2::ZERO, r as f32, Color::GRAY.alpha(0.2));
    }
}

const CENTROID_COLOR: Color = Color::GREEN;
const CENTER_OF_MASS_COLOR: Color = Color::RED;
const ORIGIN_COLOR: Color = Color::BLUE;

fn draw_trackers(d: &mut RaylibDrawHandle, trackers: &Components<Tracker>, detailed: bool) {
    for tracker in trackers.values() {
        let series = tracker.series();

        if detailed {
            let colors = [ORIGIN_COLOR, CENTER_OF_MASS_COLOR, CENTROID_COLOR];
            for (s, c) in series.iter().zip(colors) {
                let strip: Vec<_> = s.iter().map(|p| glam_to_raylib_swap_y(*p)).collect();
                d.draw_line_strip(&strip, c)
            }
        } else {
            let s = tracker.center_of_mass();
            let strip: Vec<_> = s.iter().map(|p| glam_to_raylib_swap_y(*p)).collect();
            d.draw_line_strip(&strip, Color::GRAY.alpha(0.7));
        }
    }
}

fn draw_hovered_inventory(d: &mut RaylibDrawHandle, world: &World, client: &ClientSpecificInfo) {
    let _free = some_or_return!(client.viewport.free());
    let loc = some_or_return!(client.hovered_grid_loc());
    let grid = ok_or_return!(world.grids.try_get(loc.grid_id));
    let occ = some_or_return!(grid.get_parts_at(loc.coord));
    let part_id = some_or_return!(occ.at_layer(PartLayer::Internal));
    let part = ok_or_return!(world.parts.try_get(part_id));
    let inv = ok_or_return!(world.inventories.try_get(part_id));
    let local = part.region.to_local(loc.coord);
    let slot_id = some_or_return!(inv.get_slot_at(local));
    let slot = some_or_return!(inv.get_slot(slot_id));

    let part_iso = grid.origin() * part.region.origin_isometry();
    draw_inventory_slot(d, slot, part_iso);
}

fn draw_inventory_slot(d: &mut RaylibDrawHandle, slot: &InvSlot, part_iso: Isometry2d) {
    let (min, max) = slot.location();
    let avg = (max + min).to_meters() / 2.0;
    let center_iso = part_iso.offset(avg);

    let dims = (max - min).inner().as_uvec2();
    let pl = GridRegion::new(min, Rotation::East, dims);
    let color = if let Some(item) = slot.item() {
        let c = item.color();
        Color::new(c[0], c[1], c[2], 200)
    } else {
        Color::GRAY.alpha(0.6)
    };
    draw_grid_region(d, part_iso, pl, color, Color::BLACK, slot.fill_percentage());

    let font_size = (pl.grid_aligned_dims().to_meters().min_element() / 5.0).max(0.08);

    if let Some(item) = slot.item() {
        let text = format!("{:?}", item);
        draw_text_centered_bg(d, center_iso, &text, font_size);
    }
}

fn draw_inventories(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    inventories: &Components<Inventory>,
) {
    for grid in grids.values() {
        let origin = grid.origin();
        for part_id in &grid.parts {
            let inv = ok_or_continue!(inventories.try_get(*part_id));
            let part = ok_or_continue!(parts.try_get(*part_id));
            let part_iso = origin * part.region.origin_isometry();

            for slot in inv.slots() {
                draw_inventory_slot(d, slot, part_iso);
            }
        }
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

pub fn draw_world(
    world: &World,
    client: &ClientSpecificInfo,
    assets: &Assets,
    gui: &ImGui,
    d: &mut RaylibDrawHandle,
) {
    draw_stars(d, &world.stars, &client.camera, client.screen_dims);

    let raylib_camera = to_raylib_camera(&client.camera, client.screen_dims);

    draw_players(d, &world, &raylib_camera);

    // this apparently is incredibly slow; curious
    let mut c = d.begin_mode2D(raylib_camera);

    draw_origin_and_range_indicators(&mut c);

    draw_asteroids(&mut c, world, assets, &client);

    if client.viewport.is_real_view() {
        draw_computer_target_isometry(&mut c, &world.computers, &world.parts, &world.grids);
    }

    draw_excavators(&mut c, world);

    draw_parts(
        &mut c,
        &world.grids,
        &world.parts,
        &world.prototypes,
        &raylib_camera,
        &client.viewport,
        assets,
    );

    if client.viewport.is_real_view() {
        draw_selection_info(&mut c, &world.grids, &client);

        draw_lights(
            &mut c,
            &world.grids,
            &world.parts,
            &world.computers,
            &world.lights,
            world.ticks as u32,
            &client.camera,
        );

        draw_thruster_plumes(
            &mut c,
            &world.grids,
            &world.parts,
            &world.thrusters,
            &client.camera,
        );

        draw_trackers(&mut c, &world.tracking, client.alt_mode);
    } else if let Viewport::Editor(e) = &client.viewport {
        draw_grid_lines(&mut c, &world.grids, e);
    }

    if client.alt_mode {
        draw_grid_outlines(&mut c, &world.grids);
        draw_thruster_classification(&mut c, &world.grids, &world.parts, &world.thrusters);
    }

    draw_outline_hovered_parts(&mut c, &world.grids, &world.parts, &client);

    draw_mouse_world_position(
        &mut c,
        client.mouse_screen_position,
        &client.camera,
        client.screen_dims,
    );

    draw_waypoint_widget(&mut c, client);

    draw_ping_particles(&mut c, &world.particles, world.ticks);

    // draw_isometry_axes(&mut c, world.camera.isometry, "CAM", Vec2::splat(5.0));
    // draw_isometry_axes(&mut c, world.target_camera.isometry, "", Vec2::splat(5.0));

    draw_editor_part(&mut c, world, client);

    draw_selected_part_cursors(&mut c, world, client);

    draw_hovered_part_cursor(&mut c, world, client);

    draw_editor_selection_region(&mut c, client, world);

    if client.alt_mode {
        draw_pipes(&mut c, &world.grids, &world.parts, &world.pipes);
        draw_inventories(&mut c, &world.grids, &world.parts, &world.inventories);
    } else {
        draw_hovered_inventory(&mut c, world, client);
    }

    // draw_parts_zoo(&world.prototypes, &mut c);

    drop(c);

    draw_grid_far_indicators(&world.grids, d, &client, &raylib_camera, assets);

    draw_waypoint_far_indicators(&world.computers, d, &raylib_camera);

    draw_primary_grid_inventory_summary(d, world, client);

    draw_selected_grid_info(d, &client, &world.grids, client.screen_dims, assets);

    draw_chat(d, &client.chat, client.screen_dims, assets);

    draw_imgui(d, gui, assets);

    // draw_animation(d, assets);

    // draw_item_menu(d, (300, 200).into());

    // draw_test_isos(&mut d)
}

fn draw_player_piloting(
    d: &mut RaylibDrawHandle,
    world: &World,
    camera: &Camera2D,
    name: &str,
    grid_id: Ent,
) -> BaryResult<()> {
    let grid = world.grids.try_get(grid_id)?;
    let radius = grid.bounding_radius() * camera.zoom * 1.3;
    let p = glam_to_raylib_swap_y(grid.particle_location.translation);
    let q = d.get_world_to_screen2D(p, camera);

    let font_size = 18;

    let x = q.x as i32;
    let y = q.y as i32;

    let radius = radius as i32;
    let w = radius;
    let h = radius;

    d.draw_circle_lines(x, y, radius as f32, Color::RED);

    let rec = Rectangle::new((x - w / 2) as f32, (y - h / 2) as f32, w as f32, h as f32);
    d.draw_rectangle_lines_ex(rec, 4.0, Color::RED);

    let x = x + w / 2 + 6;
    let y = y + h / 2;

    d.draw_text(&name, x, y, font_size, Color::RED);

    Ok(())
}

fn draw_players(d: &mut RaylibDrawHandle, world: &World, camera: &Camera2D) {
    for player in world.players.values() {
        match &player.state {
            PlayerState::Flying(iso) => {
                let p = glam_to_raylib_swap_y(iso.translation);
                let q = d.get_world_to_screen2D(p, camera);
                let x = q.x as i32;
                let y = q.y as i32;
                d.draw_circle(x, y, 3.0, Color::TEAL);
                d.draw_text(&player.name, x + 2, y + 6, 18, Color::TEAL);
            }
            PlayerState::PilotingGrid(grid_id, _ctrl) => {
                _ = draw_player_piloting(d, world, camera, &player.name, *grid_id);
            }
        }

        if let Some(pos) = player.cursor_world_position {
            let p = glam_to_raylib_swap_y(pos);
            let q = d.get_world_to_screen2D(p, camera);
            d.draw_circle_lines_v(q, 8.0, Color::TEAL);
        }
    }
}

fn draw_excavator(
    d: &mut RaylibDrawHandle,
    part_id: Ent,
    ex: &Excavator,
    world: &World,
) -> BaryResult<()> {
    let part = world.parts.try_get(part_id)?;
    let grid = world.grids.try_get(part.grid_id)?;
    let part_iso = grid.origin() * part.region.center_isometry();
    draw_circle(d, part_iso.translation, ex.radius, Color::RED);

    let tiles = get_excavator_tiles(part_id, ex, world)?;

    let Some((ast_id, tiles)) = tiles else {
        return Ok(());
    };

    let ast = world.asteroids.try_get(ast_id)?;
    let tile_dims = Vec2::splat(TERRAIN_TILE_WIDTH_METERS);

    for tile in tiles {
        let o = tile.origin_isometry();
        fill_rectangle(d, ast.iso * o, tile_dims, Color::TEAL.alpha(0.5));
    }

    Ok(())
}

fn draw_excavators(d: &mut RaylibDrawHandle, world: &World) {
    for (part_id, ex) in world.excavators.iter() {
        _ = draw_excavator(d, *part_id, ex, world);
    }
}

fn get_terrain_tile_rect(material: TerrainMaterial, variant: usize) -> Rectangle {
    let x = PIXELS_IN_TERRAIN_TILE as usize * variant;
    let y = PIXELS_IN_TERRAIN_TILE as usize * material as usize;
    Rectangle::new(
        x as f32,
        y as f32,
        PIXELS_IN_TERRAIN_TILE as f32,
        PIXELS_IN_TERRAIN_TILE as f32,
    )
}

#[allow(unused)]
fn draw_animation(d: &mut RaylibDrawHandle, assets: &Assets) {
    if let Some(anim) = &assets.terrain_spritesheet {
        d.draw_texture(&anim, 100, 100, Color::WHITE);

        let i = d.get_time() * 10.0;

        let rec = Rectangle::new(i as f32, 0.0, 100.0, 100.0);

        d.draw_texture_rec(&anim, rec, Vector2::new(100.0, 250.0), Color::WHITE);
    }
}

pub fn draw_selected_part_cursors(
    d: &mut RaylibDrawHandle,
    world: &World,
    client: &ClientSpecificInfo,
) {
    let free = some_or_return!(client.viewport.free());
    for loc in &free.selection_info.selected {
        let grid = ok_or_continue!(world.grids.try_get(loc.grid_id));
        let occ = some_or_continue!(grid.get_parts_at(loc.coord));
        for (_layer, id) in occ.iter() {
            let part = ok_or_continue!(world.parts.try_get(id));
            draw_grid_region(
                d,
                grid.origin(),
                part.region,
                Color::WHITE.alpha(0.0),
                Color::ORANGE,
                1.0,
            );
        }
    }
}

pub fn draw_editor_selection_region(
    d: &mut RaylibDrawHandle,
    client: &ClientSpecificInfo,
    world: &World,
) {
    let editor = some_or_return!(client.viewport.editor());
    let grid = ok_or_return!(world.grids.try_get(editor.vehicle));
    let hovered = some_or_return!(editor.hovered);
    let select_start = some_or_return!(editor.select_start);
    let start = grid.origin().offset(select_start.to_meters());
    draw_rectangle(
        d,
        start,
        Vec2::splat(PartCoord::CELL_WIDTH),
        Color::TEAL,
        0.03,
    );
    let end = grid.origin().offset(hovered.to_meters());
    draw_rectangle(
        d,
        end,
        Vec2::splat(PartCoord::CELL_WIDTH),
        Color::TEAL,
        0.03,
    );
    draw_line(d, start.translation, end.translation, Color::TEAL);
}

pub fn draw_grid_coord(
    d: &mut RaylibDrawHandle,
    grid: &VehicleGrid,
    coord: PartCoord,
    color: Color,
) {
    let iso = grid.origin().offset(coord.to_meters());
    draw_rectangle(d, iso, Vec2::splat(PartCoord::CELL_WIDTH), color, 0.03);
}

pub fn draw_grid_lattice_point(
    d: &mut RaylibDrawHandle,
    grid: &VehicleGrid,
    coord: PartCoord,
    color: Color,
) {
    let dims = Vec2::splat(PartCoord::CELL_WIDTH * 0.5);
    let iso = grid.origin().offset(coord.to_meters() - dims / 2.0);
    draw_rectangle(d, iso, dims, color, 0.03);
}

pub fn draw_hovered_part_cursor(
    d: &mut RaylibDrawHandle,
    world: &World,
    client: &ClientSpecificInfo,
) {
    let gridloc = some_or_return!(client.hovered_grid_loc());
    let grid = ok_or_return!(world.grids.try_get(gridloc.grid_id));

    let occ = grid
        .get_parts_at(gridloc.coord)
        .unwrap_or(&PartOccupancy::EMPTY);

    let color = if occ.has_any() {
        Color::GREEN
    } else {
        Color::GRAY
    };

    draw_grid_coord(d, grid, gridloc.coord, color);
}

pub fn draw_mouse_screen_position(d: &mut RaylibDrawHandle, mouse_screen_position: Option<Vec2>) {
    if let Some(pos) = mouse_screen_position {
        d.draw_circle(pos.x as i32, pos.y as i32, 4.0, Color::GRAY);
    }
}

fn draw_chat(d: &mut RaylibDrawHandle, chat: &Chat, screen_dims: Vec2, assets: &Assets) {
    let font_size = 22f32;
    let x = 10;
    let mut y = screen_dims.y - font_size - 10.0;

    for log in chat.logs() {
        let t = format_log(log);

        let pos = Vector2::new(x as f32, y as f32);

        if let Some(font) = &assets.lato_regular {
            d.draw_text_ex(font, &t, pos, font_size, 0.0, Color::WHITE);
        } else {
            d.draw_text_ex(d.get_font_default(), &t, pos, font_size, 0.0, Color::WHITE);
        }
        y -= font_size;
    }
}

pub fn draw_text_centered(
    d: &mut RaylibDrawHandle,
    font: &Font,
    text: &str,
    pos: Vector2,
    font_size: i32,
    color: Color,
) {
    if text.is_empty() {
        return;
    }

    let spacing = 1.0;
    let dims = font.measure_text(&text, font_size as f32, spacing);
    let text_origin = Vector2::new(pos.x - dims.x / 2.0, pos.y - dims.y / 2.0);

    d.draw_text_ex(
        font,
        &text,
        text_origin,
        font_size as f32,
        spacing as f32,
        color,
    );
}

pub fn draw_text_centered_weak(
    d: &mut RaylibDrawHandle,
    font: &WeakFont,
    text: &str,
    pos: Vector2,
    font_size: i32,
    color: Color,
) {
    if text.is_empty() {
        return;
    }

    let spacing = 1.0;
    let dims = font.measure_text(&text, font_size as f32, spacing);
    let text_origin = Vector2::new(pos.x - dims.x / 2.0, pos.y - dims.y / 2.0);

    d.draw_text_ex(
        font,
        &text,
        text_origin,
        font_size as f32,
        spacing as f32,
        color,
    );
}

fn draw_grid_region(
    d: &mut RaylibDrawHandle,
    root: Isometry2d,
    pl: GridRegion,
    fill: Color,
    border: Color,
    fill_pct: f32,
) {
    let bottom_left = pl.bottom_left().to_meters();
    let dims = pl.grid_aligned_dims().to_meters();
    let mut iso = root;
    iso.translation += iso.local_x() * bottom_left.x + iso.local_y() * bottom_left.y;

    let mut partial_dims = dims;
    partial_dims.x *= fill_pct;

    fill_rectangle(d, iso, partial_dims, fill);
    draw_rectangle(d, iso, dims, border, 0.03);
}

fn draw_outline_hovered_parts(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    client: &ClientSpecificInfo,
) {
    let loc = some_or_return!(client.hovered_grid_loc());
    let grid = ok_or_return!(grids.try_get(loc.grid_id));
    let occ = some_or_return!(grid.get_parts_at(loc.coord));
    let origin = grid.origin();

    for (layer, id) in occ.iter() {
        let border_color = match layer {
            PartLayer::Internal => Color::WHITE,
            PartLayer::Exterior => Color::PURPLE,
            PartLayer::Structural => Color::RED,
            _ => continue,
        };

        if let Ok(part) = parts.try_get(id) {
            draw_grid_region(
                d,
                origin,
                part.region,
                Color::PURPLE.alpha(0.3),
                border_color,
                1.0,
            );
        }
    }
}

fn draw_stars(
    d: &mut RaylibDrawHandle,
    stars: &Components<Star>,
    camera: &Camera,
    screen_dims: Vec2,
) {
    for star in stars.values() {
        let xy = Vec2::new(star.pos.x, star.pos.y);
        let offset = (xy - camera.isometry.translation) * star.pos.z;
        let offset = offset.with_y(-offset.y);
        let offset = camera.zoom * rotate(offset, camera.isometry.rotation) + screen_dims / 2.0;
        d.draw_pixel(
            offset.x as i32,
            offset.y as i32,
            Color::WHITE.alpha(star.alpha),
        );
    }
}

fn draw_primary_grid_inventory_summary(
    d: &mut RaylibDrawHandle,
    world: &World,
    client: &ClientSpecificInfo,
) {
    let free = some_or_return!(client.viewport.free());
    let sel_loc = some_or_return!(free.selection_info.selected.first());
    let grid = ok_or_return!(world.grids.try_get(sel_loc.grid_id));
    let coord = if let Some(hover) = free.selection_info.hovered {
        if hover.grid_id == sel_loc.grid_id {
            Some(hover.coord)
        } else {
            None
        }
    } else {
        None
    };

    let occ = coord
        .map(|c| grid.get_parts_at(c))
        .flatten()
        .unwrap_or(&PartOccupancy::EMPTY);

    let bar_width = 350;
    let small_bar_height = 5;
    let large_bar_height = 20;
    let highlight_width = 7;
    let bar_spacing = 1;

    let origin_x = d.get_render_width() - bar_width;
    let mut y = bar_spacing;

    for part_id in &grid.parts {
        let inv = ok_or_continue!(world.inventories.try_get(*part_id));
        for slot in inv.slots() {
            let item = slot.item();
            let c = item.map(|i| i.color()).unwrap_or([30, 30, 30]);
            let color = Color::new(c[0], c[1], c[2], 255);
            let width = bar_width as f32 * slot.fill_percentage();
            let is_hovered = occ.contains(*part_id);

            let bar_height = if is_hovered {
                large_bar_height
            } else {
                small_bar_height
            };

            d.draw_rectangle(origin_x, y, bar_width, bar_height, Color::BLACK);
            d.draw_rectangle(origin_x, y, width as i32, bar_height, color);

            if is_hovered {
                d.draw_rectangle(
                    origin_x - highlight_width,
                    y,
                    highlight_width,
                    bar_height,
                    Color::WHITE,
                );
            }

            y += bar_height + bar_spacing;
            d.draw_line(origin_x, y, origin_x + bar_width, y, Color::SLATEGRAY);
        }
    }
}

fn draw_selected_grid_info(
    d: &mut RaylibDrawHandle,
    client: &ClientSpecificInfo,
    grids: &Components<VehicleGrid>,
    screen_dims: Vec2,
    assets: &Assets,
) {
    let grid_id = some_or_return!(client.focused_grid_id());
    let grid = ok_or_return!(grids.try_get(grid_id));

    let font_size = 26;

    let parts_label = format!("{} parts - {}", grid.parts.len(), grid.parts_mass);
    let title_label = if let Some(bp) = &grid.blueprint {
        format!("{}-{} / \"{}\"", bp.0.to_uppercase(), bp.1, grid.name)
    } else {
        format!("\"{}\"", grid.name)
    };
    let pos_label = format!("{:0.2} m", grid.particle_location.translation);
    let vel_label = format!("{:0.2} m/s", grid.velocity.translation);
    let acc_label = format!("{:0.2} m/s^2", grid.linear_acceleration());

    let hx = screen_dims.x / 2.0;

    let labels = [
        (title_label, hx, 20.0),
        (parts_label, hx, 50.0),
        (pos_label, hx, screen_dims.y - 50.0),
        (vel_label, hx + 300.0, screen_dims.y - 50.0),
        (acc_label, hx - 300.0, screen_dims.y - 50.0),
    ];

    if let Some(font) = &assets.lato_regular {
        for (label, x, y) in labels {
            let pos = Vector2::new(x, y);
            draw_text_centered(d, font, &label, pos, font_size, Color::WHITE);
        }
    }
}

pub fn draw_blueprint(bp: &Blueprint, isometry: Isometry2d, d: &mut RaylibDrawHandle) {
    for draw_layer in PartLayer::draw_order() {
        let color = match draw_layer {
            PartLayer::Exterior => Color::WHITE,
            PartLayer::Internal => Color::BLUE,
            PartLayer::Plumbing => continue,
            PartLayer::Structural => Color::GRAY,
        };
        for (_, part) in bp.parts() {
            if part.layer() != draw_layer {
                continue;
            }

            let iso = part_isometry(isometry, part.region);

            let dims = part.region.part_aligned_dims().to_meters();
            fill_rectangle(d, iso, dims, color.alpha(0.4));
        }
    }
}

pub fn is_zoomed_out(camera: &Camera2D) -> bool {
    camera.zoom > 0.1
}

pub fn draw_thruster_classification(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    thrusters: &Components<Thruster>,
) {
    for grid in grids.values() {
        let root = grid.origin();
        for thruster_id in &grid.thrusters {
            let part = ok_or_continue!(parts.try_get(*thruster_id));
            let thruster = ok_or_continue!(thrusters.try_get(*thruster_id));
            let part_iso = part.region.center_isometry();
            let wrench = body_frame_wrench(
                thruster.thrust,
                part_iso.translation,
                part.region.rot(),
                grid.center_of_mass,
            );

            let color = if !thruster.is_rcs {
                Color::TEAL
            } else {
                if wrench.rotation > 0.0 {
                    Color::RED
                } else {
                    Color::GREEN
                }
            };

            draw_grid_region(d, root, part.region, color.alpha(0.7), Color::MAROON, 1.0);
        }
    }
}

pub fn draw_grid_outlines(d: &mut RaylibDrawHandle, grids: &Components<VehicleGrid>) {
    for grid in grids.values() {
        let origin = grid.origin();
        let pose = grid.particle_location;
        let centroid = grid.centroid_isometry();
        let bottom_left = PartCoord::new(grid.vehicle_bounds.0).to_meters();
        let top_right = PartCoord::new(grid.vehicle_bounds.1).to_meters();
        let bl_iso = origin.offset(bottom_left);
        let dims = top_right - bottom_left;
        draw_rectangle(d, bl_iso, dims, Color::WHITE, 0.03);

        let markers = [
            (origin, ORIGIN_COLOR),
            (centroid, CENTROID_COLOR),
            (pose, CENTER_OF_MASS_COLOR),
        ];

        for (p, _color) in markers {
            fill_circle(d, p.translation, 0.11, Color::BLACK);
        }

        for (p, c) in markers {
            draw_circle(d, p.translation, 0.1, c);
        }

        for (p, c) in markers {
            fill_circle(d, p.translation, 0.05, c);
        }

        // axes
        let origin = grid.origin();
        let x_axis = origin.offset(Vec2::X * 10.0).translation;
        let y_axis = origin.offset(Vec2::Y * 10.0).translation;
        draw_line(d, origin.translation, x_axis, Color::RED);
        draw_line(d, origin.translation, y_axis, Color::GREEN);
        draw_circle(d, origin.translation, 0.15, Color::BLUE);
    }
}

pub fn draw_editor_part(d: &mut RaylibDrawHandle, world: &World, client: &ClientSpecificInfo) {
    let editor = some_or_return!(client.viewport.editor());
    let proto_id = some_or_return!(editor.prototype_id);
    let coord = some_or_return!(editor.hovered);
    let proto = ok_or_return!(world.prototypes.try_get(proto_id));
    let grid_pose = some_or_return!(get_grid_origin(&world.grids, editor.vehicle));
    let cl = proto.classification();
    let pl = GridRegion::new(coord, editor.part_rotation, proto.dims);
    draw_part(d, pl, cl, grid_pose, false, false);
    draw_part_arrow(d, pl, grid_pose);
}

pub fn get_hovered_prototype(client: &ClientSpecificInfo, world: &World) -> Option<Ent> {
    let editor = client.viewport.editor()?;
    let layer = editor.layer?;
    let coord = editor.hovered?;
    let grid = world.grids.try_get(editor.vehicle).ok()?;
    let occ = grid.get_parts_at(coord)?;
    let part_id = occ.at_layer(layer)?;
    let part = world.parts.try_get(part_id).ok()?;
    Some(part.prototype)
}

pub fn draw_part(
    d: &mut RaylibDrawHandle,
    pl: GridRegion,
    cl: PartClassification,
    grid_isometry: Isometry2d,
    grayed: bool,
    border: bool,
) {
    let class_color = match cl {
        PartClassification::Cargo => Color::GREEN,
        PartClassification::Machine => Color::PURPLE,
        PartClassification::Thruster => Color::MAROON,
        PartClassification::Auxiliary => Color::YELLOW,
        PartClassification::DockingPort => Color::ORANGE,
        PartClassification::Computer => Color::RED,
        PartClassification::Structure => Color::GRAY.alpha(0.7),
        PartClassification::Decoration => Color::WHITE.alpha(0.7),
        PartClassification::Other => Color::GRAY,
    };

    let color = if grayed {
        Color::GRAY.alpha(0.2)
    } else {
        class_color
    };

    let iso = grid_isometry * pl.origin_isometry();
    let dims = pl.part_aligned_dims().to_meters();
    fill_rectangle(d, iso, dims, color);

    if border {
        draw_rectangle(d, iso, dims, Color::WHITE, 0.03);
    }
}

pub fn draw_part_arrow(d: &mut RaylibDrawHandle, pl: GridRegion, grid_isometry: Isometry2d) {
    let center = grid_isometry * pl.center_isometry();
    let dims = pl.part_aligned_dims().to_meters();
    let length = 0.2;
    let bottom_mid = center.offset(Vec2::X * (dims.x / 2.0 + length / 8.0));
    let v1 = glam_to_raylib_swap_y(bottom_mid.offset(Vec2::Y * length).translation);
    let v2 = glam_to_raylib_swap_y(bottom_mid.offset(-Vec2::Y * length).translation);
    let v3 = glam_to_raylib_swap_y(bottom_mid.offset(Vec2::X * length).translation);

    d.draw_triangle(v1, v2, v3, Color::GREENYELLOW);
}

pub fn draw_parts(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    protos: &Components<PartPrototype>,
    camera: &Camera2D,
    viewport: &Viewport,
    assets: &Assets,
) {
    if camera.zoom < 2.0 {
        return;
    }

    let (focus_vehicle, focus_layer) = if let Viewport::Editor(e) = viewport {
        (Some(e.vehicle), e.layer)
    } else {
        (None, None)
    };

    for (grid_id, grid) in grids.iter() {
        let is_focus_vehicle = focus_vehicle == Some(*grid_id) || focus_vehicle.is_none();

        for draw_layer in PartLayer::draw_order() {
            for part_id in &grid.parts {
                let Ok(part) = parts.try_get(*part_id) else {
                    continue;
                };

                let Ok(proto) = protos.try_get(part.prototype) else {
                    continue;
                };

                if part.layer != draw_layer {
                    continue;
                }

                let is_focus_layer = Some(part.layer) == focus_layer || focus_layer.is_none();

                let is_shown = is_focus_layer && is_focus_vehicle;

                // draw_part(
                //     d,
                //     part.region,
                //     part.classification,
                //     origin,
                //     !is_shown,
                //     is_shown && focus_layer.is_some(),
                // );

                let tint = if is_shown {
                    Color::WHITE
                } else {
                    Color::WHITE.alpha(0.3)
                };

                if let Some(sprite) = assets.part_textures.get(&proto.name) {
                    let part_iso = match part.region.rot() {
                        Rotation::East => part.region.top_left_isometry(),
                        Rotation::North => part.region.bottom_left_isometry(),
                        Rotation::South => part.region.top_right_isometry(),
                        Rotation::West => part.region.bottom_right_isometry(),
                    };

                    let iso = grid.origin() * part_iso;
                    let pos = glam_to_raylib_swap_y(iso.translation);
                    let rot = -iso.rotation.to_degrees();
                    let scale = 1.0 / 20.0;
                    d.draw_texture_ex(sprite, pos, rot, scale, tint);
                }
            }
        }
    }
}

pub fn draw_computer_target_isometry(
    d: &mut RaylibDrawHandle,
    computers: &Components<Computer>,
    parts: &Components<Part>,
    grids: &Components<VehicleGrid>,
) {
    for (cpu_id, cpu) in computers.iter() {
        let Ok(part) = parts.try_get(*cpu_id) else {
            continue;
        };

        let Ok(grid) = grids.try_get(part.grid_id) else {
            continue;
        };

        let Some(pose) = cpu.current_waypoint() else {
            continue;
        };

        draw_isometry_axes(d, pose, &grid.name, Vec2::new(1.0, 0.4));
    }
}

pub fn draw_grid_blueprints(d: &mut RaylibDrawHandle, world: &World, camera: &Camera2D) {
    for (grid_id, grid) in world.grids.iter() {
        if camera.zoom > 0.1 {
            let Ok(bp) = get_blueprint(world, *grid_id) else {
                continue;
            };
            let origin = grid.origin();
            draw_blueprint(&bp, origin, d);
            // draw_isometry_axes(d, grid.pose, &grid.name);
            let s = format!("{} / {}", grid.parts.len(), grid.parts_mass);
            draw_text(d, origin, &s, 1.5);
        }
    }
}

#[allow(unused)]
fn draw_test_isos(d: &mut RaylibDrawHandle) {
    let test_isos = [
        (
            Color::RED,
            Isometry2d::new((10.0, 20.0).into(), 40.0f32.to_radians()),
        ),
        (
            Color::GREEN,
            Isometry2d::new((40.0, 12.0).into(), -10.0f32.to_radians()),
        ),
        (
            Color::BLUE,
            Isometry2d::new((70.0, 50.0).into(), d.get_time() as f32),
        ),
    ];

    for (color, iso) in test_isos {
        let dims = Vec2::new(10.0, 4.0);
        fill_rectangle(d, iso, dims, color.alpha(0.5));
        draw_isometry_axes(d, iso, "TST", Vec2::splat(8.0));
    }
}

fn draw_ping_particles(d: &mut RaylibDrawHandle, particles: &Vec<PingParticle>, current_tick: u64) {
    for particle in particles {
        let wavelength = 1.0;
        let ticks_per_wave = 25;
        let n_waves = 10;
        let vel = wavelength / ticks_per_wave as f32;
        let init_offset = (current_tick - particle.start_tick()) % ticks_per_wave;

        let r0 = init_offset as f32 * vel;

        let rmax = wavelength * n_waves as f32;

        for r in linspace(r0, r0 + rmax, n_waves) {
            let a = (1.0 - r / rmax).clamp(0.0, 1.0);
            draw_circle(d, particle.pos(), r, Color::ORANGE.alpha(a));
        }
    }
}

fn draw_waypoint_far_indicators(
    computers: &Components<Computer>,
    d: &mut RaylibDrawHandle,
    camera: &Camera2D,
) {
    if camera.zoom > ZOOM_NEAR_FAR_THRESHOLD {
        return;
    }

    let marker_radius = 11.0f32;

    for cpu in computers.values() {
        if !cpu.on {
            continue;
        }

        let Some(wp) = cpu.current_waypoint() else {
            continue;
        };

        let pos = glam_to_raylib_swap_y(wp.translation);
        let pos = d.get_world_to_screen2D(pos, camera);
        d.draw_circle_lines_v(pos, marker_radius, Color::GRAY);
    }
}

fn draw_imgui(d: &mut RaylibDrawHandle, gui: &ImGui, assets: &Assets) {
    const DRAW_OUTLINES: bool = false;

    const BACKGROUND_COLOR: Color = Color::new(20, 20, 20, 255);
    const BUTTON_IDLE_COLOR: Color = Color::new(40, 40, 40, 255);
    const BUTTON_HOVERED_COLOR: Color = Color::new(50, 50, 50, 255);
    const BUTTON_PRESSED_COLOR: Color = Color::new(120, 70, 70, 255);

    for layout in &gui.layouts {
        let p = layout.origin;
        let s = layout.dims;
        d.draw_rectangle(p.x, p.y, s.x, s.y, BACKGROUND_COLOR);

        if layout.id == gui.active && DRAW_OUTLINES {
            let rect = Rectangle::new(p.x as f32, p.y as f32, s.x as f32, s.y as f32);
            d.draw_rectangle_lines_ex(rect, 2.0, Color::RED);
        }

        for b in &layout.text_areas {
            let color = if b.is_pressed {
                BUTTON_PRESSED_COLOR
            } else if b.id == gui.active {
                BUTTON_HOVERED_COLOR
            } else {
                BUTTON_IDLE_COLOR
            };

            let p = b.origin;

            {
                let mut p = p;
                let mut dims = b.dims;
                if b.is_pressed {
                    let n = 4;
                    p += IVec2::splat(n);
                    dims -= IVec2::splat(n * 2);
                }
                d.draw_rectangle(p.x, p.y, dims.x, dims.y, color);
            }

            let font_size = 24.0;

            if let Some(font) = &assets.lato_regular {
                let tdims = font.measure_text(&b.text, font_size, 0.0);
                let t = glam_to_raylib(p.as_vec2()) + glam_to_raylib(b.dims.as_vec2()) / 2.0
                    - tdims / 2.0;

                d.draw_text_ex(font, &b.text, t, font_size, 0.0, Color::WHITE);
            }

            if b.id == gui.active && DRAW_OUTLINES {
                let rect = Rectangle::new(p.x as f32, p.y as f32, b.dims.x as f32, b.dims.y as f32);
                d.draw_rectangle_lines_ex(rect, 2.0, Color::RED);
            }
        }
    }

    if gui.active == 0 && DRAW_OUTLINES {
        let rect = Rectangle::new(0.0, 0.0, gui.screen.x, gui.screen.y);
        d.draw_rectangle_lines_ex(rect, 2.0, Color::RED);
    }
}

fn draw_waypoint_widget(d: &mut RaylibDrawHandle, client: &ClientSpecificInfo) {
    let free = some_or_return!(client.viewport.free());
    if free.selection_info.selected.is_empty() {
        return;
    }
    let start = some_or_return!(free.waypoint_widget);
    let screen_pos = some_or_return!(client.mouse_screen_position);
    let end = screen_to_world(&client.camera, screen_pos, client.screen_dims);
    draw_line(d, start, end, Color::GRAY);

    let n_ships = free.selection_info.selected.len();

    for i in 0..n_ships {
        let s = if n_ships == 1 {
            1.0
        } else {
            i as f32 / (n_ships - 1) as f32
        };

        let p = start.lerp(end, s);

        draw_circle(d, p, 10.0 / client.camera.zoom, Color::GRAY)
    }
}

fn draw_mouse_world_position(
    d: &mut RaylibDrawHandle,
    mouse_screen_position: Option<Vec2>,
    camera: &Camera,
    screen_dims: Vec2,
) {
    let Some(screen_pos) = mouse_screen_position else {
        return;
    };

    let world_pos = screen_to_world(camera, screen_pos, screen_dims);
    let r = 10.0 / camera.zoom;
    draw_circle(d, world_pos, r, Color::WHITE);
}

fn draw_line(d: &mut RaylibDrawHandle, start: Vec2, end: Vec2, color: Color) {
    let start = glam_to_raylib_swap_y(start);
    let end = glam_to_raylib_swap_y(end);
    d.draw_line_v(start, end, color);
}

fn draw_line_width(d: &mut RaylibDrawHandle, start: Vec2, end: Vec2, thick: f32, color: Color) {
    let start = glam_to_raylib_swap_y(start);
    let end = glam_to_raylib_swap_y(end);
    d.draw_line_ex(start, end, thick, color);
}

#[allow(unused)]
fn draw_parts_zoo(parts: &Components<PartPrototype>, d: &mut RaylibDrawHandle) {
    let x = 0;
    let mut y = 0;
    for proto in parts.values() {
        // if let Some(t) = texture {
        //     d.draw_texture_ex(
        //         t,
        //         Vector2::new(x as f32, y as f32),
        //         0.0,
        //         1.0 / 5.0,
        //         Color::WHITE,
        //     );
        // }

        let rect = Rectangle::new(x as f32, y as f32, proto.dims.x as f32, proto.dims.y as f32);

        d.draw_rectangle_lines_ex(rect, 0.3, Color::TEAL.alpha(0.7));

        y += proto.dims.y as i32 + 1;
    }
}

fn draw_rectangle(d: &mut RaylibDrawHandle, iso: Isometry2d, dims: Vec2, color: Color, t: f32) {
    let xoff = iso.local_x() * dims.x;
    let yoff = iso.local_y() * dims.y;
    let w = glam_to_raylib_swap_y(iso.translation);
    let x = glam_to_raylib_swap_y(iso.translation + xoff);
    let y = glam_to_raylib_swap_y(iso.translation + xoff + yoff);
    let z = glam_to_raylib_swap_y(iso.translation + yoff);
    for window in [w, x, y, z, w].windows(2) {
        d.draw_line_ex(window[0], window[1], t, color);
    }
}

fn fill_rectangle(d: &mut RaylibDrawHandle, iso: Isometry2d, dims: Vec2, color: Color) {
    let rec = Rectangle::new(iso.translation.x, -iso.translation.y, dims.x, dims.y);
    let origin = Vector2::new(0.0, dims.y);
    let rotation = -iso.rotation.to_degrees();
    d.draw_rectangle_pro(rec, origin, rotation, color);
}

fn fill_circle(d: &mut RaylibDrawHandle, p: Vec2, r: f32, color: Color) {
    let center = glam_to_raylib_swap_y(p);
    d.draw_circle_v(center, r, color);
}

fn draw_circle(d: &mut RaylibDrawHandle, p: Vec2, r: f32, color: Color) {
    let center = glam_to_raylib_swap_y(p);
    d.draw_circle_lines_v(center, r, color);
}

fn draw_selection_info(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    client: &ClientSpecificInfo,
) {
    let free = some_or_return!(client.viewport.free());
    if let Some(gridloc) = free.selection_info.hovered {
        let grid = ok_or_return!(grids.try_get(gridloc.grid_id));
        let loc = grid.centroid_isometry();
        let r = grid.bounding_radius() * 1.4;
        draw_circle(d, loc.translation, r, Color::GREEN);
    }
    for loc in &free.selection_info.selected {
        let grid = ok_or_return!(grids.try_get(loc.grid_id));
        let loc = grid.centroid_isometry();
        let r = grid.bounding_radius() * 1.4 + 0.5;
        draw_circle(d, loc.translation, r, Color::ORANGE);
    }
}

fn get_pipe_joint_location(
    joint: PipeJoint,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
) -> BaryResult<Isometry2d> {
    let part = parts.try_get(joint.part_id)?;
    let grid = grids.try_get(part.grid_id)?;
    let grid_root = grid.origin();
    let part_root = part.region.origin_isometry();
    let offset = joint.offset.to_meters();
    Ok(grid_root * part_root.offset(offset))
}

fn draw_pipes(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    pipes: &Components<Pipe>,
) {
    for pipe in pipes.values() {
        let src = ok_or_continue!(get_pipe_joint_location(pipe.src, grids, parts));
        let dst = ok_or_continue!(get_pipe_joint_location(pipe.dst, grids, parts));

        let dims = Vec2::new(0.6, 0.6) * PartCoord::CELL_WIDTH;

        let half_cell = Vec2::splat(PartCoord::CELL_WIDTH) / 2.0;
        let offset = half_cell - dims / 2.0;

        let p = src.offset(half_cell).translation;
        let q = dst.offset(half_cell).translation;

        let color = match pipe.status {
            MachineStatus::NoRoom => Color::ORANGE,
            MachineStatus::Starved => Color::BLUE,
            _ => Color::TEAL,
        };

        draw_rectangle(d, src.offset(offset), dims, Color::GREEN, 0.03);
        draw_rectangle(d, dst.offset(offset), dims, Color::RED, 0.03);
        draw_line_width(d, p, q, 0.04, color);
    }
}

fn draw_grid_lines(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    editor: &EditorState,
) {
    let grid = ok_or_return!(grids.try_get(editor.vehicle));
    let iso = grid.origin();
    let dims = grid.dims().to_meters();

    let x = (-2, dims.x.ceil() as i32 + 2);
    let y = (-2, dims.y.ceil() as i32 + 2);

    for x in x.0..=x.1 {
        let start = iso.offset(Vec2::new(x as f32, y.0 as f32)).translation;
        let end = iso.offset(Vec2::new(x as f32, y.1 as f32)).translation;
        draw_line(d, start, end, Color::GRAY.alpha(0.5));
    }
    for y in y.0..=y.1 {
        let start = iso.offset(Vec2::new(x.0 as f32, y as f32)).translation;
        let end = iso.offset(Vec2::new(x.1 as f32, y as f32)).translation;
        draw_line(d, start, end, Color::GRAY.alpha(0.5));
    }
}

fn draw_thruster_plumes(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    thrusters: &Components<Thruster>,
    camera: &Camera,
) {
    if camera.zoom < 2.0 {
        return;
    }

    for (e, t) in thrusters.iter() {
        if !t.is_on {
            continue;
        }

        let Ok(part) = parts.try_get(*e) else {
            continue;
        };

        let Ok(grid) = grids.try_get(part.grid_id) else {
            continue;
        };

        let origin = grid.origin();

        let mut iso = part_isometry(origin, part.region);
        let mut dims = part.region.part_aligned_dims().to_meters();
        let p = iso.translation + iso.local_y() * dims.y / 2.0;
        dims.x *= 2.0;
        let offset = iso.local_x() * dims.x;
        iso.translation -= offset;

        fill_rectangle(d, iso, dims, Color::RED);
        draw_light_source(d, p, dims.x, Color::RED);
    }
}

fn draw_lights(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    computers: &Components<Computer>,
    lights: &Components<Light>,
    ticks: u32,
    camera: &Camera,
) {
    if camera.zoom < 2.0 {
        return;
    }

    for grid in grids.values() {
        let Some(cpu_id) = grid.computers.first() else {
            continue;
        };

        let Ok(cpu) = computers.try_get(*cpu_id) else {
            continue;
        };

        if !cpu.on {
            continue;
        }

        let origin = grid.origin();

        for light_id in &grid.lights {
            let Ok(light) = lights.try_get(*light_id) else {
                continue;
            };
            let Ok(part) = parts.try_get(*light_id) else {
                continue;
            };

            let rate = 3;

            if !light.is_on(ticks * rate) {
                continue;
            }

            let light_isometry = origin * part.region.center_isometry();
            fill_rectangle(d, light_isometry, Vec2::splat(0.1), Color::ORANGE);
            draw_light_source(d, light_isometry.translation, 0.1, Color::YELLOW);
        }
    }
}

fn draw_light_source(d: &mut RaylibDrawHandle, p: Vec2, r_scale: f32, color: Color) {
    for r in [1.0f32, 1.5, 3.0] {
        let r = r_scale * r.powi(2);
        let a = 0.2 * 1.0 / r;
        let color = color.alpha(a);
        fill_circle(d, p, r, color);
    }
}

#[allow(unused)]
fn draw_item_menu(d: &mut RaylibDrawHandle, origin: IVec2) {
    let n_cols: i32 = 5;

    let item_width = 24;
    let padding = 9;
    let margin = 10;

    let mut x = origin.x;
    let mut y = origin.y;
    let mut n_col = 0;

    let n_items = Item::all().count() as i32;
    let n_rows = n_items / n_cols + 1;

    {
        let bg_color = Color::new(40, 40, 40, 220);
        let bx = origin.x - margin;
        let by = origin.y - margin;
        let wx = 2 * margin + n_cols * (item_width + padding) - padding;
        let wy = 2 * margin + n_rows * (item_width + padding) - padding;
        d.draw_rectangle(bx, by, wx, wy, bg_color);
    }

    for item in Item::all() {
        let color = item.color();
        let color = Color::new(color[0], color[1], color[2], 255);
        d.draw_rectangle(x, y, item_width, item_width, color);

        x += item_width + padding;
        n_col += 1;

        if n_col == n_cols {
            x = origin.x;
            y += item_width + padding;
            n_col = 0;
        }
    }
}

fn draw_terrain_chunk(
    d: &mut RaylibDrawHandle,
    rock: &BigRock,
    chunk_index: ChunkIndex,
    chunk_id: Ent,
    world: &World,
    spritesheet: &Texture2D,
) -> bool {
    let Ok(chunk) = world.terrain_chunks.try_get(chunk_id) else {
        return false;
    };

    if chunk.visible_count == 0 {
        return false;
    }

    let iso = rock.iso * chunk_index.origin_isometry();

    let mut drew_tiles = false;

    for (tile_index, tile_id) in &chunk.tiles {
        let tile = ok_or_continue!(world.terrain_tiles.try_get(*tile_id));
        if tile.light_level() == 0 {
            continue;
        }
        drew_tiles = true;
        let top_left = iso * tile_index.top_left_isometry();
        let p = glam_to_raylib_swap_y(top_left.translation);
        let rot = -top_left.rotation.to_degrees();
        let scale = TERRAIN_TILE_WIDTH_METERS;
        let source_rec = get_terrain_tile_rect(tile.material(), tile.variant());
        let dest_rec = Rectangle::new(p.x, p.y, scale, scale);
        let alpha = tile.light_level() as f32 / 10.0;
        d.draw_texture_pro(
            spritesheet,
            source_rec,
            dest_rec,
            Vector2::zero(),
            rot,
            Color::WHITE.alpha(alpha),
        );
    }

    drew_tiles
}

fn is_chunk_onscreen(rock_iso: Isometry2d, chunk: ChunkIndex, client: &ClientSpecificInfo) -> bool {
    let dims = client.screen_dims;
    let bb = chunk.bb();
    for corner in bb.corners().iter().chain(&[bb.center]) {
        let w = rock_iso * Isometry2d::from_pos(*corner);
        let s = get_world_to_screen(&client.camera, w.translation, dims);
        if s.min_element() > 0.0 && s.y < dims.y && s.x < dims.x {
            return true;
        }
    }
    // TODO stopgap; implement better BB collisions
    for corner in bb.corners().iter().chain(&[bb.center]) {
        let corner = (corner - bb.center) * 0.5 + bb.center;
        let w = rock_iso * Isometry2d::from_pos(corner);
        let s = get_world_to_screen(&client.camera, w.translation, dims);
        if s.min_element() > 0.0 && s.y < dims.y && s.x < dims.x {
            return true;
        }
    }
    false
}

fn draw_asteroid(
    d: &mut RaylibDrawHandle,
    rock: &BigRock,
    assets: &Assets,
    world: &World,
    client: &ClientSpecificInfo,
) {
    let spritesheet = assets
        .terrain_spritesheet
        .as_ref()
        .expect("No terrain spritesheet");

    let mut points = Vec::new();
    for theta in linspace(0.0, PI * 2.0, 200) {
        let r = rock.ast.radius_at(theta);
        let theta = theta + rock.iso.rotation;
        let p = r * rotate(Vec2::X, theta);
        let p = glam_to_raylib_swap_y(rock.iso.translation + p);
        points.push(p);
    }
    d.draw_line_strip(&points, Color::WHITE);

    let mut chunks_to_draw = Vec::new();

    if client.camera.zoom > 1.4 {
        let camera_bb = AABB::from_arbitrary(Vec2::ZERO, client.screen_dims);
        let mut bounds: Option<(ChunkIndex, ChunkIndex)> = None;

        for corner in camera_bb.corners() {
            let w = screen_to_world(&client.camera, corner, client.screen_dims);
            let wrt_asteroid = in_frame(rock.iso, w);
            let chunk_index = ChunkIndex(vfloor(wrt_asteroid / TERRAIN_CHUNK_WIDTH_METERS));

            if let Some((min, max)) = &mut bounds {
                min.0.x = min.0.x.min(chunk_index.0.x);
                min.0.y = min.0.y.min(chunk_index.0.y);
                max.0.x = max.0.x.max(chunk_index.0.x);
                max.0.y = max.0.y.max(chunk_index.0.y);
            } else {
                bounds = Some((chunk_index, chunk_index));
            }
        }

        if let Some((min, max)) = bounds {
            for x in min.0.x..=max.0.x {
                for y in min.0.y..=max.0.y {
                    let c = ChunkIndex((x, y).into());
                    if is_chunk_onscreen(rock.iso, c, &client) {
                        if let Some(id) = rock.chunks.get(&c) {
                            chunks_to_draw.push((c, *id));
                        }
                    }
                }
            }
        }

        const MAX_CHUNKS_TO_DRAW: u32 = 60;
        let mut n_drawn = 0;

        for (chunk_index, chunk_id) in &chunks_to_draw {
            n_drawn +=
                draw_terrain_chunk(d, rock, *chunk_index, *chunk_id, world, spritesheet) as u32;
            if n_drawn == MAX_CHUNKS_TO_DRAW {
                break;
            }
        }
    }

    if client.alt_mode {
        draw_circle(d, rock.iso.translation, rock.ast.min_radius(), Color::RED);
        draw_circle(d, rock.iso.translation, rock.ast.base_radius(), Color::BLUE);
        draw_circle(d, rock.iso.translation, rock.ast.max_radius(), Color::GREEN);

        let o = rock.iso.translation;
        let x = o + rock.iso.local_x() * rock.ast.base_radius();
        let y = o + rock.iso.local_y() * rock.ast.base_radius();

        draw_line(d, o, x, Color::RED);
        draw_line(d, o, y, Color::GREEN);

        let t = 0.4 / client.camera.zoom;

        for (chunk_index, id) in chunks_to_draw {
            let chunk = ok_or_continue!(world.terrain_chunks.try_get(id));
            let iso = rock.iso * chunk_index.origin_isometry();
            let dims = Vec2::splat(TERRAIN_CHUNK_WIDTH_METERS);
            let color = if chunk.visible_count > 0 {
                Color::RED
            } else {
                Color::RED.alpha(0.2)
            };
            draw_rectangle(d, iso, dims, color, t);
        }
    }
}

fn draw_asteroids(
    d: &mut RaylibDrawHandle,
    world: &World,
    assets: &Assets,
    client: &ClientSpecificInfo,
) {
    for (_id, rock) in world.asteroids.iter() {
        draw_asteroid(d, rock, assets, world, client);
    }
}

fn severity_to_color(s: LogLevel) -> Color {
    match s {
        LogLevel::Debug => Color::GRAY.alpha(0.5),
        LogLevel::Warning => Color::YELLOW,
        LogLevel::Error => Color::RED,
        LogLevel::Info => Color::GRAY,
        LogLevel::Terminal => Color::ORANGE,
        LogLevel::Command => Color::WHITE,
    }
}

pub fn draw_terminal<T: std::fmt::Debug>(
    d: &mut RaylibDrawHandle,
    cmd: &Terminal<T>,
    assets: &Assets,
    bg_color: Color,
) {
    if !cmd.is_active() {
        return;
    };

    let Some(font) = &assets.consolas else {
        return;
    };

    d.draw_rectangle(0, 0, d.get_render_width(), d.get_render_height(), bg_color);

    let chars = cmd.display_text();

    let fg: String = chars
        .iter()
        .map(|(c, b)| if *b { *c } else { ' ' })
        .collect();

    let bg: String = chars
        .iter()
        .map(|(c, b)| if !*b { *c } else { ' ' })
        .collect();

    let display = format!("bsh > {}", fg);
    let display_bg = format!("bsh > {}", bg);
    // let width = d.get_render_width();
    let height = d.get_render_height();

    let padding = 14;
    let line_gap = 0;
    let font_size = cmd.font_size() as i32;

    let text_origin = IVec2::new(padding, height - padding - font_size);

    for (i, (line, severity)) in cmd.lines().enumerate() {
        let origin = text_origin - IVec2::Y * (font_size + line_gap) * (i as i32 + 1);
        d.draw_text_ex(
            font,
            &line,
            Vector2::new(origin.x as f32, origin.y as f32),
            font_size as f32,
            1.0,
            severity_to_color(*severity),
        );
    }

    {
        d.draw_text_ex(
            font,
            &display_bg,
            Vector2::new(text_origin.x as f32, text_origin.y as f32),
            font_size as f32,
            1.0,
            Color::GRAY.alpha(0.6),
        );

        d.draw_text_ex(
            font,
            &display,
            Vector2::new(text_origin.x as f32, text_origin.y as f32),
            font_size as f32,
            1.0,
            Color::WHITE,
        );
    }
}
