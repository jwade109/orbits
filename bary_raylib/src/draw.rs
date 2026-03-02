use std::collections::BTreeSet;

use crate::camera::{Camera, to_raylib_camera};
use crate::components::Components;
use crate::light::Light;
use crate::part::*;
use crate::result::BaryResult;
use crate::ring_particle::*;
use crate::systems::*;
use crate::thruster::Thruster;
use crate::utils::*;
use crate::vehicle_grid::*;
use crate::world::SelectionInfo;
use crate::world::World;
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

fn draw_isometry_axes(d: &mut RaylibDrawHandle, iso: Isometry2d, label: &str) {
    let x = iso.translation + iso.local_x() * 10.0;
    let y = iso.translation + iso.local_y() * 7.0;

    let p = glam_to_raylib_swap_y(iso.translation);
    let x = glam_to_raylib_swap_y(x);
    let y = glam_to_raylib_swap_y(y);

    d.draw_circle_v(p, 0.1, Color::WHITE);
    d.draw_circle_v(x, 0.1, Color::RED);
    d.draw_circle_v(y, 0.1, Color::GREEN);

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

pub fn draw_world(world: &World, d: &mut RaylibDrawHandle) {
    let raylib_camera = to_raylib_camera(&world.camera, world.screen_dims);

    // this apparently is incredibly slow; curious
    let mut c = d.begin_mode2D(raylib_camera);

    draw_origin_and_range_indicators(&mut c);

    // draw_grid_blueprints(
    //     &mut c,
    //     &world.grids,
    //     &world.parts,
    //     &world.prototypes,
    //     &world.camera,
    // );

    draw_parts(&mut c, &world.grids, &world.parts, &raylib_camera);
    draw_thrusters(&mut c, &world.grids, &world.parts, &world.thrusters);
    draw_lights(&mut c, &world.grids, &world.lights);

    _ = draw_selection_info(&mut c, &world.grids, &world.selection_info);

    draw_grids_if_updated_this_frame(&mut c, &world.grids_to_update, &world.grids);

    draw_focused_grid_cursor(
        &mut c,
        &world.grids,
        &world.parts,
        world.mouse_screen_position,
        &world.camera,
        world.screen_dims,
        &world.selection_info,
    );

    draw_mouse_world_position(
        &mut c,
        world.mouse_screen_position,
        &world.camera,
        world.screen_dims,
    );

    draw_particles(&mut c, &world.particles);

    // draw_isometry_axes(&mut c, world.camera.isometry, "CAM");
    // draw_isometry_axes(&mut c, world.target_camera.isometry, "");

    drop(c);

    draw_grid_far_indicators(&world.grids, d, &raylib_camera);

    draw_selected_grid_info(d, &world.selection_info, &world.grids, world.screen_dims);

    // draw_parts_zoo(&world.prototypes, &mut d);
    // draw_test_isos(&mut d)
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
    mouse_screen_position: Option<Vec2>,
    camera: &Camera,
    screen_dims: Vec2,
    sel: &SelectionInfo,
) {
    let Some(screen_pos) = mouse_screen_position else {
        return;
    };
    let Some((id, _offset)) = sel.selected else {
        return;
    };
    let Ok(grid) = grids.try_get(id) else {
        return;
    };

    let world_pos = screen_to_world(camera, screen_pos, screen_dims);

    let mut iso = grid.isometry;

    let coord = PartCoord::from_meters_floored(in_frame(iso, world_pos));

    let color = if grid.has_part_at(coord) {
        Color::GREEN.alpha(0.8)
    } else {
        Color::RED.alpha(0.8)
    };

    for (_part_id, part) in get_parts_at(grid, parts, coord) {
        draw_grid_placement(
            d,
            grid.isometry,
            part.placement,
            Color::PURPLE.alpha(0.3),
            Color::WHITE,
        );
    }

    let offset = coord.to_meters();

    iso.translation += iso.local_x() * offset.x;
    iso.translation += iso.local_y() * offset.y;

    let size = PartCoord::CELL_WIDTH;

    let x_axis = grid.isometry.translation + iso.local_x() * 10.0;
    let y_axis = grid.isometry.translation + iso.local_y() * 10.0;

    draw_line(d, grid.isometry.translation, x_axis, Color::RED);
    draw_line(d, grid.isometry.translation, y_axis, Color::GREEN);
    draw_circle(d, grid.isometry.translation, 0.15, Color::BLUE);

    fill_rectangle(d, iso, Vec2::splat(size), color);
}

fn draw_selected_grid_info(
    d: &mut RaylibDrawHandle,
    sel: &SelectionInfo,
    grids: &Components<VehicleGrid>,
    screen_dims: Vec2,
) {
    let Some((id, _offset)) = sel.selected else {
        return;
    };
    let Ok(grid) = grids.try_get(id) else {
        return;
    };

    let font_size = 26;

    let title_label = format!("\"{}\"", grid.name);
    let parts_label = format!("{} parts - {}", grid.parts.len(), grid.parts_mass);
    let vel_label = format!("{:0.2} m/s", grid.linear_velocity);

    let labels = [
        (title_label, screen_dims.x / 2.0, 20.0),
        (parts_label, screen_dims.x / 2.0, 50.0),
        (vel_label, screen_dims.x / 2.0, screen_dims.y - 50.0),
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

pub fn draw_parts(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    camera: &Camera2D,
) {
    for grid in grids.values() {
        if camera.zoom > 2.0 {
            for draw_layer in PartLayer::draw_order() {
                let color = match draw_layer {
                    PartLayer::Exterior => Color::WHITE,
                    PartLayer::Internal => Color::BLUE,
                    PartLayer::Plumbing => continue,
                    PartLayer::Structural => Color::GRAY,
                };
                for part_id in &grid.parts {
                    let Ok(part) = parts.try_get(*part_id) else {
                        continue;
                    };

                    if part.layer != draw_layer {
                        continue;
                    }

                    let iso = part_isometry(grid.isometry, part.placement);
                    let dims = part.placement.part_aligned_dims().to_meters();
                    fill_rectangle(d, iso, dims, color);
                }
            }
        }
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
            draw_blueprint(&bp, grid.isometry, d);
            // draw_isometry_axes(d, grid.isometry, &grid.name);
            let s = format!("{} / {}", grid.parts.len(), grid.parts_mass);
            draw_text(d, grid.isometry, &s);
        }
    }
}

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
        draw_isometry_axes(d, iso, "TST");
    }
}

fn draw_particles(d: &mut RaylibDrawHandle, particles: &Vec<RingParticle>) {
    for particle in particles {
        let r = particle.radius();
        fill_circle(d, particle.pos, r, particle.color());
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

    let marker_radius = 30.0f32;

    let mut markers = Vec::new();

    for grid in grids.values() {
        let p = glam_to_raylib_swap_y(grid.isometry.translation);
        let q = d.get_world_to_screen2D(p, camera);

        markers.push((q, q, &grid.name));
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

    // draw the markers
    for (p, q, name) in markers {
        d.draw_line_v(p, q, Color::ORANGE);
        d.draw_circle_lines_v(q, marker_radius, Color::ORANGE);
        if !name.is_empty() {
            let q = q + Vector2::new(marker_radius + 10.0, 0.0);
            d.draw_text_ex(
                d.get_font_default(),
                name,
                q,
                24.0,
                0.4,
                Color::ORANGE.alpha(0.5),
            );
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
    if let Some((grid_id, _)) = sel.camera_hovered {
        let grid = grids.try_get(grid_id)?;
        draw_circle(d, grid.isometry.translation, 15.0, Color::RED);
    }
    if let Some((grid_id, _)) = sel.mouse_hovered {
        let grid = grids.try_get(grid_id)?;
        draw_circle(d, grid.isometry.translation, 16.0, Color::GREEN);
    }
    if let Some((grid_id, _)) = sel.selected {
        let grid = grids.try_get(grid_id)?;
        draw_circle(d, grid.isometry.translation, 17.0, Color::BLUE);
        draw_circle(d, grid.isometry.translation, 18.0, Color::BLUE);
    }
    Ok(())
}

fn draw_thrusters(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    thrusters: &Components<Thruster>,
) {
    for (e, t) in thrusters.iter() {
        if !t.is_on {
            continue;
        }

        let Ok(grid) = grids.try_get(t.grid_id) else {
            continue;
        };
        let Ok(part) = parts.try_get(*e) else {
            continue;
        };

        let mut iso = part_isometry(grid.isometry, part.placement);
        let mut dims = part.placement.part_aligned_dims().to_meters();
        let p = iso.translation + iso.local_y() * dims.y / 2.0;
        dims.x *= 2.0;
        let offset = iso.local_x() * dims.x;
        iso.translation -= offset;

        fill_rectangle(d, iso, dims, Color::RED);
        draw_light_source(d, p, dims.x, Color::RED);
    }
}

fn draw_grids_if_updated_this_frame(
    d: &mut RaylibDrawHandle,
    updates: &BTreeSet<Ent>,
    grids: &Components<VehicleGrid>,
) {
    for e in updates {
        let Ok(grid) = grids.try_get(*e) else {
            continue;
        };
        draw_circle(d, grid.isometry.translation, 10.0, Color::PURPLE);
    }
}

fn draw_lights(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    lights: &Components<Light>,
) {
    for light in lights.values() {
        let Some(grid) = grids.get(light.grid_id) else {
            continue;
        };

        if light.is_on() {
            let offset = grid.isometry.local_x() * light.position.x
                + grid.isometry.local_y() * light.position.y;
            let mut light_isometry = grid.isometry;
            light_isometry.translation += offset;

            fill_rectangle(d, light_isometry, Vec2::splat(0.1), Color::ORANGE);
            draw_light_source(d, light_isometry.translation, 1.0, Color::YELLOW);
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
