use crate::camera::{Camera, to_raylib_camera};
use crate::chat::{Chat, format_log};
use crate::components::Components;
use crate::result::BaryResult;
use crate::ring_particle::*;
use crate::systems::*;
use crate::ui::{Window, draw_window};
use crate::utils::*;
use crate::vehicle::*;
use crate::world::{Assets, ClientSpecificInfo, SelectionInfo};
use crate::world::{Tracker, World};
use bary_core::prelude::PI;
use bary_core::prelude::*;
use raylib::prelude::*;

fn draw_text(d: &mut RaylibDrawHandle, iso: Isometry2d, text: &str) {
    let p = glam_to_raylib_swap_y(iso.translation);
    if !text.is_empty() {
        d.draw_text_pro(
            d.get_font_default(),
            &text,
            p,
            Vector2::zero(),
            -iso.rotation.to_degrees(),
            1.5,
            0.1,
            Color::ORANGE,
        );
    }
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

    draw_text(d, iso, label);
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

fn draw_trackers(d: &mut RaylibDrawHandle, trackers: &Components<Tracker>) {
    for tracker in trackers.values() {
        let series = tracker.series();
        let colors = [ORIGIN_COLOR, CENTER_OF_MASS_COLOR, CENTROID_COLOR];
        for (s, c) in series.iter().zip(colors) {
            let strip: Vec<_> = s.iter().map(|p| glam_to_raylib_swap_y(*p)).collect();
            d.draw_line_strip(&strip, c)
        }
    }
}

pub fn draw_world(
    world: &World,
    client: &ClientSpecificInfo,
    assets: &Assets,
    d: &mut RaylibDrawHandle,
) {
    let raylib_camera = to_raylib_camera(&world.camera, client.screen_dims);

    // this apparently is incredibly slow; curious
    let mut c = d.begin_mode2D(raylib_camera);

    draw_origin_and_range_indicators(&mut c);

    draw_computer_target_isometry(&mut c, &world.computers, &world.grids);

    // draw_grid_blueprints(
    //     &mut c,
    //     &world.grids,
    //     &world.parts,
    //     &world.prototypes,
    //     &world.camera,
    // );

    draw_parts(&mut c, &world.grids, &world.parts, &raylib_camera);
    draw_thrusters(
        &mut c,
        &world.grids,
        &world.parts,
        &world.thrusters,
        &world.camera,
    );
    draw_lights(
        &mut c,
        &world.grids,
        &world.parts,
        &world.computers,
        &world.lights,
        world.ticks as u32,
        &world.camera,
    );

    draw_grid_outlines(&mut c, &world.grids);

    _ = draw_selection_info(&mut c, &world.grids, &world.selection_info);

    draw_focused_grid_cursor(&mut c, &world.grids, &world.parts, &world.selection_info);

    draw_mouse_world_position(
        &mut c,
        client.mouse_screen_position,
        &world.camera,
        client.screen_dims,
    );

    draw_particles(&mut c, &world.particles);

    // draw_isometry_axes(&mut c, world.camera.isometry, "CAM");
    // draw_isometry_axes(&mut c, world.target_camera.isometry, "");

    draw_trackers(&mut c, &world.tracking);

    drop(c);

    draw_grid_far_indicators(&world.grids, d, &raylib_camera);

    draw_waypoint_far_indicators(&world.computers, d, &raylib_camera);

    draw_selected_grid_info(d, &world.selection_info, &world.grids, client.screen_dims);

    draw_chat(d, &client.chat, client.screen_dims, assets);

    draw_selected_grid_primary_computer_info(d, world, assets);

    draw_hovered_part_info(d, world, assets);

    // draw_parts_zoo(&world.prototypes, &mut d);
    // draw_test_isos(&mut d)
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

fn draw_text_centered(d: &mut RaylibDrawHandle, text: &str, pos: Vector2, font_size: i32) {
    let width = d.measure_text(&text, font_size);
    let pos = Vector2::new(pos.x - width as f32 / 2.0, pos.y);

    d.draw_text_ex(
        d.get_font_default(),
        &text,
        pos,
        font_size as f32,
        3.0,
        Color::WHITE,
    );
}

fn draw_grid_placement(
    d: &mut RaylibDrawHandle,
    root: Isometry2d,
    pl: GridPlacement,
    fill: Color,
    border: Color,
) {
    let bottom_left = pl.bottom_left().to_meters();
    let dims = pl.grid_aligned_dims().to_meters();
    let mut iso = root;
    iso.translation += iso.local_x() * bottom_left.x + iso.local_y() * bottom_left.y;
    fill_rectangle(d, iso, dims, fill);
    draw_rectangle(d, iso, dims, border);
}

fn draw_focused_grid_cursor(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    sel: &SelectionInfo,
) {
    let Some(grid_id) = sel.selected_grid else {
        return;
    };
    let Ok(grid) = grids.try_get(grid_id) else {
        return;
    };

    let size = PartCoord::CELL_WIDTH;

    let origin = grid.origin();

    for info in [sel.mouseover_part_info] {
        let Some((coord, occ)) = info else {
            continue;
        };

        // cursor
        let iso = origin.offset(coord.to_meters());
        fill_rectangle(d, iso, Vec2::splat(size), Color::GREEN);

        for (_layer, id) in occ.iter() {
            if let Ok(part) = parts.try_get(id) {
                draw_grid_placement(
                    d,
                    origin,
                    part.placement,
                    Color::PURPLE.alpha(0.3),
                    Color::WHITE,
                );
            }
        }
    }

    // axes
    let origin = grid.origin();
    let x_axis = origin.offset(Vec2::X * 10.0).translation;
    let y_axis = origin.offset(Vec2::Y * 10.0).translation;
    draw_line(d, origin.translation, x_axis, Color::RED);
    draw_line(d, origin.translation, y_axis, Color::GREEN);
    draw_circle(d, origin.translation, 0.15, Color::BLUE);
}

fn draw_selected_grid_info(
    d: &mut RaylibDrawHandle,
    sel: &SelectionInfo,
    grids: &Components<VehicleGrid>,
    screen_dims: Vec2,
) {
    let Some(grid_id) = sel.selected_grid else {
        return;
    };
    let Ok(grid) = grids.try_get(grid_id) else {
        return;
    };

    let font_size = 26;

    let title_label = format!("\"{}\"", grid.name);
    let parts_label = format!("{} parts - {}", grid.parts.len(), grid.parts_mass);
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

    for (label, x, y) in labels {
        let pos = Vector2::new(x, y);
        draw_text_centered(d, &label, pos, font_size);
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

            let iso = part_isometry(isometry, part.placement);

            let dims = part.placement.part_aligned_dims().to_meters();
            fill_rectangle(d, iso, dims, color.alpha(0.4));
        }
    }
}

pub fn is_zoomed_out(camera: &Camera2D) -> bool {
    camera.zoom > 0.1
}

pub fn draw_grid_outlines(d: &mut RaylibDrawHandle, grids: &Components<VehicleGrid>) {
    for grid in grids.values() {
        let origin = grid.origin();
        let pose = grid.particle_location;
        let centroid = grid.centroid_isometry();
        let bottom_left = PartCoord::new(grid.bounds.0).to_meters();
        let top_right = PartCoord::new(grid.bounds.1).to_meters();
        let bl_iso = origin.offset(bottom_left);
        let dims = top_right - bottom_left;
        draw_rectangle(d, bl_iso, dims, Color::WHITE);

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
    }
}

pub fn draw_parts(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    camera: &Camera2D,
) {
    if camera.zoom < 2.0 {
        return;
    }

    for grid in grids.values() {
        let origin = grid.origin();
        for draw_layer in PartLayer::draw_order() {
            for part_id in &grid.parts {
                let Ok(part) = parts.try_get(*part_id) else {
                    continue;
                };

                if part.layer != draw_layer {
                    continue;
                }

                let color = match part.classification {
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

                let iso = part_isometry(origin, part.placement);
                let dims = part.placement.part_aligned_dims().to_meters();
                fill_rectangle(d, iso, dims, color);
            }
        }
    }
}

pub fn draw_computer_target_isometry(
    d: &mut RaylibDrawHandle,
    computers: &Components<Computer>,
    grids: &Components<VehicleGrid>,
) {
    for cpu in computers.values() {
        let Ok(grid) = grids.try_get(cpu.grid_id) else {
            continue;
        };
        draw_isometry_axes(d, cpu.pose, &grid.name, Vec2::new(5.0, 3.0));
    }
}

pub fn draw_grid_blueprints(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    prototypes: &Components<PartPrototype>,
    camera: &Camera2D,
) {
    for (grid_id, grid) in grids.iter() {
        if camera.zoom > 0.1 {
            let Ok(bp) = get_blueprint(grids, parts, prototypes, *grid_id) else {
                continue;
            };
            let origin = grid.origin();
            draw_blueprint(&bp, origin, d);
            // draw_isometry_axes(d, grid.pose, &grid.name);
            let s = format!("{} / {}", grid.parts.len(), grid.parts_mass);
            draw_text(d, origin, &s);
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

fn draw_particles(d: &mut RaylibDrawHandle, particles: &Vec<PingParticle>) {
    for particle in particles {
        let r = particle.radius();
        if particle.is_visible() {
            draw_circle(d, particle.pos, r, particle.color());
            fill_circle(d, particle.pos, r / 10.0, particle.color());
        }
    }
}

fn draw_waypoint_far_indicators(
    computers: &Components<Computer>,
    d: &mut RaylibDrawHandle,
    camera: &Camera2D,
) {
    if camera.zoom > 7.0 {
        return;
    }

    let marker_radius = 11.0f32;

    for cpu in computers.values() {
        if !cpu.on {
            continue;
        }

        let pos = glam_to_raylib_swap_y(cpu.pose.translation);
        let pos = d.get_world_to_screen2D(pos, camera);
        d.draw_circle_lines_v(pos, marker_radius, Color::GRAY);
    }
}

fn draw_grid_far_indicators(
    grids: &Components<VehicleGrid>,
    d: &mut RaylibDrawHandle,
    camera: &Camera2D,
) {
    if camera.zoom > 7.0 {
        return;
    }

    let marker_radius = 14.0f32;

    let mut markers = Vec::new();

    for (id, grid) in grids.iter() {
        let loc = grid.particle_location;
        let p = glam_to_raylib_swap_y(loc.translation);
        let q = d.get_world_to_screen2D(p, camera);

        let name = format!("{} {}", id, grid.name);
        markers.push((q, q, loc.rotation, name, !grid.computers.is_empty()));
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

    // draw the markers
    for (p, q, angle, name, is_controllable) in markers {
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
        d.draw_circle_lines_v(q, marker_radius, color);
        if !name.is_empty() {
            let q = q + Vector2::new(marker_radius + 10.0, 0.0);
            d.draw_text_ex(d.get_font_default(), &name, q, 24.0, 0.4, color.alpha(0.5));
        }
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

fn draw_rectangle(d: &mut RaylibDrawHandle, iso: Isometry2d, dims: Vec2, color: Color) {
    let xoff = iso.local_x() * dims.x;
    let yoff = iso.local_y() * dims.y;
    let w = glam_to_raylib_swap_y(iso.translation);
    let x = glam_to_raylib_swap_y(iso.translation + xoff);
    let y = glam_to_raylib_swap_y(iso.translation + xoff + yoff);
    let z = glam_to_raylib_swap_y(iso.translation + yoff);
    for window in [w, x, y, z, w].windows(2) {
        d.draw_line_ex(window[0], window[1], 0.05, color);
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
    sel: &SelectionInfo,
) -> BaryResult<()> {
    if let Some(grid_id) = sel.camera_hovered {
        let grid = grids.try_get(grid_id)?;
        let loc = grid.centroid_isometry();
        draw_circle(d, loc.translation, 15.0, Color::RED);
    }
    if let Some(grid_id) = sel.mouse_hovered {
        let grid = grids.try_get(grid_id)?;
        let loc = grid.centroid_isometry();
        draw_circle(d, loc.translation, 16.0, Color::GREEN);
    }
    if let Some(grid_id) = sel.selected_grid {
        let grid = grids.try_get(grid_id)?;
        let loc = grid.centroid_isometry();
        draw_circle(d, loc.translation, 17.0, Color::BLUE);
    }
    Ok(())
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
    let lines = [
        format!("CPU INFO ==="),
        format!("\n  On: {}", cpu.on),
        format!("\n  Status: {:?}", cpu.status),
        format!("\n  Ticks: {}", cpu.ticks_this_cycle),
        format!("\n  Fired: {}", cpu.fired_this_tick),
        format!("\n  Iters: {}", cpu.iters),
        format!("\n  Mode: {:?}", cpu.mode),
        format!("\n  Pose: {:?}", cpu.pose.to_tuple()),
        format!("\n  Vel: {:?}", cpu.velocity.to_tuple()),
    ];

    lines.into_iter().collect()
}

fn draw_selected_grid_primary_computer_info(
    d: &mut RaylibDrawHandle,
    world: &World,
    assets: &Assets,
) {
    let Some(grid_id) = world.selection_info.selected_grid else {
        return;
    };

    let Ok(grid) = world.grids.try_get(grid_id) else {
        return;
    };

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
        // draw_window(d, &window, font);
    }
}

fn draw_hovered_part_info(d: &mut RaylibDrawHandle, world: &World, assets: &Assets) {
    let Some((coord, occ)) = world.selection_info.mouseover_part_info else {
        return;
    };

    let mut s = format!("At {}: {:?}", coord, occ.to_array());

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
        origin: IVec2::new(200, 60),
        title: "Part Info".to_string(),
        content: s,
        is_focused: true,
    };

    if let Some(font) = &assets.fira_code {
        draw_window(d, &window, font);
    }
}

fn draw_thrusters(
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

        let mut iso = part_isometry(origin, part.placement);
        let mut dims = part.placement.part_aligned_dims().to_meters();
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

        let origin = grid.origin();

        for light_id in &grid.lights {
            let Ok(light) = lights.try_get(*light_id) else {
                continue;
            };
            let Ok(part) = parts.try_get(*light_id) else {
                continue;
            };

            let rate = if cpu.on { 4 } else { 1 };

            if !light.is_on(ticks * rate) {
                continue;
            }

            let light_isometry = origin * part.placement.center_isometry();
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
