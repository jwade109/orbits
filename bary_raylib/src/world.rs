use crate::components::*;
use crate::computer::*;
use crate::input_state::*;
use crate::thruster::*;
use crate::vehicle_grid::*;
use bary_core::prelude::PI;
use bary_core::prelude::*;
use raylib::prelude::*;
use rdev::Event;
use std::collections::VecDeque;

#[derive(Debug)]
pub struct RingParticle {
    pub pos: Vec2,
    pub age_left: f32,
}

impl RingParticle {
    pub fn radius(&self) -> f32 {
        self.age_left * 10.0
    }
}

pub type MaybeTexture = Option<Texture2D>;

pub struct World {
    pub counter: EntityCounter,
    pub snap_camera_to_local_planet: bool,
    pub screen_dims: Vector2,
    pub input: InputState,
    pub event_queue: VecDeque<Event>,
    pub camera: Camera2D,
    pub target_camera: Camera2D,
    pub particles: Vec<RingParticle>,
    pub blueprints: Components<NamedBlueprint>,
    pub prototypes: Components<(PartPrototype, MaybeTexture)>,
    pub thrusters: Components<Thruster>,
    pub computers: Components<Computer>,
    pub grids: Components<VehicleGrid>,
    pub circle_texture: MaybeTexture,
}

impl World {
    pub fn empty() -> Self {
        Self {
            counter: EntityCounter::default(),
            snap_camera_to_local_planet: false,
            screen_dims: Vector2::new(1500.0, 900.0),
            input: InputState::default(),
            event_queue: VecDeque::new(),
            camera: Camera2D {
                zoom: 0.1,
                ..default_camera_2d()
            },
            target_camera: default_camera_2d(),
            particles: Vec::default(),
            blueprints: Components::default(),
            prototypes: Components::default(),
            grids: Components::default(),
            thrusters: Components::default(),
            computers: Components::default(),
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

fn raylib_to_glam(v: Vector2) -> Vec2 {
    Vec2::new(v.x, v.y)
}

fn update_camera_target(input: &InputState, screen_dims: Vector2, target: &mut Camera2D) {
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
    }
    if input.is_key_pressed(Key::KeyE) {
        target.rotation -= angular_speed;
    }
    if input.is_key_pressed(Key::KeyS) {
        target.target += up * speed;
    }
    if input.is_key_pressed(Key::KeyW) {
        target.target -= up * speed;
    }
    if input.is_key_pressed(Key::KeyD) {
        target.target += right * speed;
    }
    if input.is_key_pressed(Key::KeyA) {
        target.target -= right * speed;
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
        let age_left = 1.0;
        let particle = RingParticle { pos, age_left };
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
    }
}

pub fn update_world(world: &mut World, screen_dims: Vector2) {
    world.screen_dims = screen_dims;

    toggle_camera_local_normal_snapping(&world.event_queue, &mut world.snap_camera_to_local_planet);
    update_ring_particles(&mut world.particles);
    update_input_state(&world.event_queue, &mut world.input);
    apply_scroll_wheel_to_camera_target(&world.event_queue, &mut world.target_camera);
    update_camera_target(&world.input, world.screen_dims, &mut world.target_camera);
    if world.snap_camera_to_local_planet {
        snap_camera_target_to_local_up(&mut world.target_camera);
    }
    update_camera(&world.target_camera, &mut world.camera);
    spawn_random_ring_effects(&mut world.particles);
    propagate_grid_rigid_bodies(&mut world.grids);
    panic_if_escape_is_pressed(&world.input);

    world.event_queue.clear();
}

fn update_ring_particles(particles: &mut Vec<RingParticle>) {
    let dt = 0.02;
    for ring in particles.iter_mut() {
        ring.age_left -= dt;
    }
    particles.retain(|p| p.age_left > 0.0);
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

fn draw_grids(grids: &Components<VehicleGrid>, d: &mut RaylibDrawHandle) {
    for grid in grids.values() {
        draw_blueprint(&grid.blueprint, grid.isometry.translation, d);
        let p = grid.isometry.translation.as_ivec2();
        d.draw_circle(p.x, p.y, 0.5, Color::RED);
        let s = format!("{}\n{}", grid.parts.len(), grid.mass);
        d.draw_text(&s, p.x, p.y, 6, Color::RED);
    }
}

pub fn draw_world(world: &World, d: &mut RaylibDrawHandle) {
    let Some(t) = &world.circle_texture else {
        return;
    };
    for particle in &world.particles {
        d.draw_texture(
            t,
            particle.pos.x as i32,
            particle.pos.y as i32,
            Color::ORANGE.alpha(0.3),
        );
    }

    d.draw_circle_lines(0, 0, 100.0, Color::GRAY);
    d.draw_circle_lines(0, 0, 95.0, Color::GRAY);

    draw_parts_zoo(&world.prototypes, d);
    draw_grids(&world.grids, d);
}

pub fn push_event(world: &mut World, event: Event) {
    world.event_queue.push_back(event);
}

pub fn draw_blueprint(bp: &Blueprint, offset: Vec2, d: &mut RaylibDrawHandle) {
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

            let bl = offset + part.placement.bottom_left().to_meters() * 10.0;
            let tr = offset + part.placement.top_right().to_meters() * 10.0;
            let rectangle = Rectangle::new(bl.x, bl.y, tr.x - bl.x, tr.y - bl.y);
            d.draw_rectangle_lines_ex(rectangle, 1.0, color);
        }
    }
}
