use crate::components::*;
use crate::computer::*;
use crate::input_state::*;
use crate::light::*;
use crate::part::*;
use crate::thruster::*;
use crate::vehicle_grid::*;
use bary_core::prelude::PI;
use bary_core::prelude::*;
use raylib::prelude::*;
use rdev::Event;
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Debug)]
pub struct RingParticle {
    pub pos: Vec2,
    pub time_left: f32,
}

impl RingParticle {
    pub fn radius(&self) -> f32 {
        self.time_left * 10.0
    }
}

pub type MaybeTexture = Option<Texture2D>;

#[derive(Default)]
pub struct Timers {
    pub update: Duration,
    pub render: Duration,
}

#[derive(Default, Debug)]
pub struct SelectionInfo {
    pub camera_hovered: Option<(EntityId, Vec2)>,
    pub mouse_hovered: Option<(EntityId, Vec2)>,
}

pub struct World {
    pub timers: Timers,
    pub mouse_screen_position: Option<Vec2>,
    pub selection_info: SelectionInfo,
    pub spawner: EntitySpawner,
    pub follow_vehicle: Option<EntityId>,
    pub snap_camera_to_local_planet: bool,
    pub screen_dims: Vector2,
    pub input: InputState,
    pub event_queue: VecDeque<Event>,
    pub camera: Camera2D,
    pub target_camera: Camera2D,
    pub particles: Vec<RingParticle>,
    pub blueprints: Components<NamedBlueprint>,
    pub prototypes: Components<(PartPrototype, MaybeTexture)>,
    pub parts: Components<Part>,
    pub thrusters: Components<Thruster>,
    pub computers: Components<Computer>,
    pub lights: Components<Light>,
    pub grids: Components<VehicleGrid>,
    pub circle_texture: MaybeTexture,
}

impl World {
    pub fn empty() -> Self {
        Self {
            timers: Timers::default(),
            mouse_screen_position: None,
            selection_info: SelectionInfo::default(),
            spawner: EntitySpawner::default(),
            snap_camera_to_local_planet: false,
            follow_vehicle: None,
            screen_dims: Vector2::new(1500.0, 900.0),
            input: InputState::default(),
            event_queue: VecDeque::new(),
            camera: Camera2D {
                zoom: 0.1,
                ..default_camera_2d()
            },
            target_camera: Camera2D {
                zoom: 8.0,
                ..default_camera_2d()
            },
            particles: Vec::default(),
            blueprints: Components::default(),
            prototypes: Components::default(),
            parts: Components::default(),
            grids: Components::default(),
            thrusters: Components::default(),
            computers: Components::default(),
            lights: Components::default(),
            circle_texture: None,
        }
    }
}

pub fn load_assets(
    world: &mut World,
    rl: &mut raylib::RaylibHandle,
    thread: &raylib::RaylibThread,
) {
    world.circle_texture = rl.load_texture(thread, "assets/circle.png").ok();

    for (proto, tex) in world.prototypes.values_mut() {
        let filename = format!("assets/parts/{}/skin.png", proto.part_name());
        *tex = rl.load_texture(thread, &filename).ok();
    }
}

fn update_input_state(events: &VecDeque<Event>, state: &mut InputState) {
    for e in events {
        if let rdev::EventType::KeyPress(k) = e.event_type {
            state.set_pressed(k);
        } else if let rdev::EventType::KeyRelease(k) = e.event_type {
            state.set_released(k);
        }
    }
}

pub fn glam_to_raylib(v: Vec2) -> Vector2 {
    Vector2::new(v.x, v.y)
}

pub fn glam_to_raylib_swap_x(v: Vec2) -> Vector2 {
    Vector2::new(-v.x, v.y)
}

pub fn glam_to_raylib_swap_y(v: Vec2) -> Vector2 {
    Vector2::new(v.x, -v.y)
}

pub fn raylib_to_glam(v: Vector2) -> Vec2 {
    Vec2::new(v.x, v.y)
}

fn raylib_to_glam_invert_y(v: Vector2) -> Vec2 {
    Vec2::new(v.x, -v.y)
}

fn update_camera_target(
    input: &InputState,
    screen_dims: Vector2,
    target: &mut Camera2D,
    follow: &mut Option<EntityId>,
) {
    target.offset = screen_dims / 2.0;

    let angular_speed = 1.5;
    let speed = 22.0 / target.zoom;
    let zoom_scale = 1.03;

    // camera rotation is stored as degrees!
    // why would raylib do this to me.
    let right = rotate(Vec2::X, -target.rotation.to_radians());
    let up = rotate(right, PI / 2.0);

    let right = glam_to_raylib(right);
    let up = glam_to_raylib(up);

    if input.is_key_pressed(Key::Minus) {
        target.zoom /= zoom_scale;
    }
    if input.is_key_pressed(Key::Equal) {
        target.zoom *= zoom_scale;
    }
    if input.is_key_pressed(Key::KeyQ) {
        target.rotation += angular_speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyE) {
        target.rotation -= angular_speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyS) {
        target.target += up * speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyW) {
        target.target -= up * speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyD) {
        target.target += right * speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyA) {
        target.target -= right * speed;
        *follow = None;
    }
}

fn update_camera(target: &Camera2D, actual: &mut Camera2D) {
    let rate_translation = 0.1;
    let rate_rotation = 0.1;
    actual.offset = target.offset;
    actual.target.x = low_pass(actual.target.x, target.target.x, rate_translation);
    actual.target.y = low_pass(actual.target.y, target.target.y, rate_translation);
    actual.rotation = low_pass(actual.rotation, target.rotation, rate_rotation);
    actual.zoom = low_pass(actual.zoom, target.zoom, rate_translation);
}

fn get_isometry(camera: &Camera2D) -> Isometry2d {
    Isometry2d {
        translation: raylib_to_glam_invert_y(camera.target),
        rotation: camera.rotation.to_radians(),
    }
}

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

fn default_camera_2d() -> Camera2D {
    Camera2D {
        offset: Vector2::zero(),
        target: Vector2::zero(),
        rotation: 0.0,
        zoom: 1.0,
    }
}

fn spawn_random_ring_effects(particles: &mut Vec<RingParticle>) {
    for _ in 0..20 {
        let pos = randvec(0.0, 10000.0);
        let time_left = 1.0;
        let particle = RingParticle { pos, time_left };
        particles.push(particle);
    }
}

fn apply_scroll_wheel_to_camera_target(events: &VecDeque<Event>, target: &mut Camera2D) {
    let scale = 1.15;
    for e in events {
        if let rdev::EventType::Wheel {
            delta_x: _,
            delta_y,
        } = e.event_type
        {
            if delta_y > 0 {
                target.zoom *= scale;
            } else if delta_y < 0 {
                target.zoom /= scale;
            }
        }
    }
}

fn panic_if_escape_is_pressed(input: &InputState) {
    if input.is_key_pressed(Key::Escape) {
        panic!();
    }
}

fn toggle_camera_local_normal_snapping(events: &VecDeque<Event>, snap_camera: &mut bool) {
    for e in events {
        if let rdev::EventType::KeyPress(Key::KeyT) = e.event_type {
            *snap_camera ^= true;
        }
    }
}

fn toggle_following_on_key_f(
    events: &VecDeque<Event>,
    sel: &SelectionInfo,
    follow: &mut Option<EntityId>,
) {
    let pressed_f = events
        .iter()
        .any(|e| e.event_type == rdev::EventType::KeyPress(Key::KeyF));

    if !pressed_f {
        return;
    }

    let Some((id, _delta)) = sel.camera_hovered else {
        return;
    };

    *follow = Some(id);
}

fn snap_camera_target_to_local_up(target: &mut Camera2D) {
    let r = 100.0;
    let p = raylib_to_glam(target.target);
    let q = if p.length() < r {
        p.normalize_or_zero() * r
    } else {
        p
    };
    target.rotation = -(q.to_angle() + PI / 2.0).to_degrees();
    target.target = glam_to_raylib(q);
}

fn propagate_grid_rigid_bodies(grids: &mut Components<VehicleGrid>) {
    let dt = 0.02;
    for grid in grids.values_mut() {
        grid.isometry.translation += grid.linear_velocity * dt;
        grid.isometry.rotation += grid.angular_velocity * dt;
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

            for r in [1.0f32, 1.5, 3.0] {
                let r = r.powi(2);
                let a = 0.2 * 1.0 / r;
                let color = Color::YELLOW.alpha(a);
                fill_circle(d, light_isometry.translation, r, color);
            }
        }
    }
}

fn update_selection_info(
    info: &mut SelectionInfo,
    grids: &Components<VehicleGrid>,
    camera: &Camera2D,
    mouse_screen_position: Option<Vec2>,
) {
    let pos = camera.target;
    let test_pos = raylib_to_glam_invert_y(pos);
    info.camera_hovered = find_closest_grid(grids, test_pos);
    // if let Some(pos) = mouse_world_position {
    //     info.mouse_hovered = find_closest_grid(grids, pos);
    // } else {
    //     info.mouse_hovered = None;
    // }
}

fn set_camera_if_following(
    follow: Option<EntityId>,
    grids: &Components<VehicleGrid>,
    target: &mut Camera2D,
    actual: &mut Camera2D,
) {
    let Some(follow) = follow else {
        return;
    };

    let Some(grid) = grids.get(follow) else {
        return;
    };

    target.target = glam_to_raylib_swap_y(grid.isometry.translation);
    target.rotation = grid.isometry.rotation.to_degrees();

    *actual = *target;
}

pub fn update_world(world: &mut World, screen_dims: Vector2, mouse_screen_position: Option<Vec2>) {
    let start = std::time::Instant::now();

    world.screen_dims = screen_dims;
    world.mouse_screen_position = mouse_screen_position;

    toggle_camera_local_normal_snapping(&world.event_queue, &mut world.snap_camera_to_local_planet);
    toggle_following_on_key_f(
        &world.event_queue,
        &world.selection_info,
        &mut world.follow_vehicle,
    );

    update_ring_particles(&mut world.particles);
    update_lights(&mut world.lights);
    update_computers(&mut world.computers);

    update_selection_info(
        &mut world.selection_info,
        &world.grids,
        &world.camera,
        world.mouse_screen_position,
    );

    update_input_state(&world.event_queue, &mut world.input);
    apply_scroll_wheel_to_camera_target(&world.event_queue, &mut world.target_camera);

    propagate_grid_rigid_bodies(&mut world.grids);

    update_camera_target(
        &world.input,
        world.screen_dims,
        &mut world.target_camera,
        &mut world.follow_vehicle,
    );

    set_camera_if_following(
        world.follow_vehicle,
        &world.grids,
        &mut world.target_camera,
        &mut world.camera,
    );

    if world.snap_camera_to_local_planet {
        snap_camera_target_to_local_up(&mut world.target_camera);
    }
    update_camera(&world.target_camera, &mut world.camera);

    // spawn_random_ring_effects(&mut world.particles);

    panic_if_escape_is_pressed(&world.input);

    world.event_queue.clear();

    let end = std::time::Instant::now();

    world.timers.update = end - start;
}

fn update_ring_particles(particles: &mut Vec<RingParticle>) {
    let dt = 0.02;
    for ring in particles.iter_mut() {
        ring.time_left -= dt;
    }
    particles.retain(|p| p.time_left > 0.0);
}

fn update_lights(lights: &mut Components<Light>) {
    for light in lights.values_mut() {
        light.ticks += 1;
    }
}

fn draw_parts_zoo(parts: &Components<(PartPrototype, MaybeTexture)>, d: &mut RaylibDrawHandle) {
    let x = 0;
    let mut y = 0;
    for (proto, texture) in parts.values() {
        if let Some(t) = texture {
            d.draw_texture_ex(
                t,
                Vector2::new(x as f32, y as f32),
                0.0,
                1.0 / 5.0,
                Color::WHITE,
            );
        }

        let rect = Rectangle::new(x as f32, y as f32, proto.dims.x as f32, proto.dims.y as f32);

        d.draw_rectangle_lines_ex(rect, 0.3, Color::TEAL.alpha(0.7));

        y += proto.dims.y as i32 + 1;
    }
}

fn camera_from_isometry(iso: Isometry2d) -> Camera2D {
    Camera2D {
        offset: Vector2::zero(),
        target: glam_to_raylib_swap_y(iso.translation),
        rotation: iso.rotation.to_degrees(),
        zoom: 1.0,
    }
}

pub fn draw_grids(
    d: &mut RaylibDrawHandle,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    prototypes: &Components<(PartPrototype, MaybeTexture)>,
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

fn draw_particles(d: &mut RaylibDrawHandle, particles: &Vec<RingParticle>, t: &Texture2D) {
    for particle in particles {
        d.draw_texture(
            t,
            particle.pos.x as i32,
            particle.pos.y as i32,
            Color::ORANGE.alpha(0.3),
        );
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
            d.draw_text_ex(d.get_font_default(), name, q, 24.0, 0.4, Color::ORANGE);
        }
    }
}

pub fn draw_world(world: &World, d: &mut RaylibDrawHandle) {
    let mut c = d.begin_mode2D(world.camera);

    let Some(t) = &world.circle_texture else {
        return;
    };

    draw_particles(&mut c, &world.particles, t);
    draw_grids(
        &mut c,
        &world.grids,
        &world.parts,
        &world.prototypes,
        &world.camera,
    );

    draw_lights(&mut c, &world.grids, &world.lights);

    drop(c);

    draw_grid_far_indicators(&world.grids, d, &world.camera);

    // draw_parts_zoo(&world.prototypes, &mut d);
    // draw_isometry_axes(&mut d, get_isometry(&world.camera), "CAM");
    // draw_isometry_axes(&mut d, get_isometry(&world.target_camera), "");
    // draw_test_isos(&mut d)
}

pub fn push_event(world: &mut World, event: Event) {
    world.event_queue.push_back(event);
}

pub fn part_isometry(root_isometry: Isometry2d, placement: GridPlacement) -> Isometry2d {
    let part_iso = placement.origin_isometry();

    // TODO replace this with std::ops::Mul
    let rotation = root_isometry.rotation + part_iso.rotation;
    let offset = root_isometry.local_x() * part_iso.translation.x
        + root_isometry.local_y() * part_iso.translation.y;
    Isometry2d::new(root_isometry.translation + offset, rotation)
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
