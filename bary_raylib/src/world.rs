use crate::components::*;
use crate::computer::*;
use crate::input_state::*;
use crate::light::*;
use crate::multiplayer::Action;
use crate::multiplayer::Transaction;
use crate::part::*;
use crate::ring_particle::RingParticle;
use crate::systems::*;
use crate::thruster::*;
use crate::utils::*;
use crate::vehicle_grid::*;
use bary_core::prelude::PI;
use bary_core::prelude::*;
use log::{debug, info};
use raylib::prelude::*;
use rdev::Event;
use serde::{Deserialize, Serialize};
use std::collections::*;
use std::time::Duration;

pub type MaybeTexture = Option<Texture2D>;

pub type MaybeFont = Option<Font>;

#[derive(Default, Deserialize, Serialize)]
pub struct Timers {
    pub update: Duration,
    pub render: Duration,
    pub total: Duration,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct SelectionInfo {
    pub camera_hovered: Option<(Ent, Vec2)>,
    pub mouse_hovered: Option<(Ent, Vec2)>,
}

#[derive(Default)]
pub struct Assets {
    pub circle_texture: MaybeTexture,
    pub lato_regular: MaybeFont,
    pub part_textures: BTreeMap<String, Texture2D>,
}

#[derive(Serialize, Deserialize)]
pub struct World {
    pub ticks: u64,
    pub timers: Timers,
    pub mouse_screen_position: Option<Vec2>,
    pub selection_info: SelectionInfo,
    pub spawner: EntitySpawner,
    pub follow_vehicle: Option<Ent>,
    pub snap_camera_to_local_planet: bool,
    pub screen_dims: Vec2,
    #[serde(skip)]
    pub input: InputState,
    #[serde(skip)]
    pub event_queue: VecDeque<Event>,
    #[serde(skip)]
    pub camera: Camera2D,
    #[serde(skip)]
    pub target_camera: Camera2D,
    pub particles: Vec<RingParticle>,
    pub blueprints: Components<NamedBlueprint>,
    pub prototypes: Components<PartPrototype>,
    pub parts: Components<Part>,
    pub thrusters: Components<Thruster>,
    pub computers: Components<Computer>,
    pub lights: Components<Light>,
    pub grids: Components<VehicleGrid>,
    pub grids_to_update: BTreeSet<Ent>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "World({}, {} grids, {})",
            self.ticks,
            self.grids.len(),
            self.spawner.next()
        )
    }
}

impl World {
    pub fn empty() -> Self {
        Self {
            ticks: 0,
            timers: Timers::default(),
            mouse_screen_position: None,
            selection_info: SelectionInfo::default(),
            spawner: EntitySpawner::default(),
            snap_camera_to_local_planet: false,
            follow_vehicle: None,
            screen_dims: Vec2::new(1500.0, 900.0),
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
            grids_to_update: BTreeSet::new(),
        }
    }
}

pub fn size_in_bytes(world: &World) -> usize {
    let bytes = bincode::serialize(world).unwrap();
    bytes.len()
}

pub fn load_assets(
    assets: &mut Assets,
    rl: &mut raylib::RaylibHandle,
    thread: &raylib::RaylibThread,
) {
    debug!("Loading assets");
    assets.circle_texture = rl.load_texture(thread, "assets/circle.png").ok();
    assets.lato_regular = rl.load_font(thread, "assets/fonts/Lato-Regular.ttf").ok();

    // for (proto, tex) in assets.part_textures.values_mut() {
    //     let filename = format!("assets/parts/{}/skin.png", proto.part_name());
    //     *tex = rl.load_texture(thread, &filename).ok();
    // }
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

fn update_camera_target(
    input: &InputState,
    screen_dims: Vec2,
    target: &mut Camera2D,
    follow: &mut Option<Ent>,
) {
    target.offset = glam_to_raylib(screen_dims / 2.0);

    let angular_speed = 2.5;
    let speed = 40.0 / target.zoom;
    let zoom_scale = 1.07;

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
    let rate_translation = 0.2;
    let rate_rotation = 0.2;
    actual.offset = target.offset;
    actual.target.x = low_pass(actual.target.x, target.target.x, rate_translation);
    actual.target.y = low_pass(actual.target.y, target.target.y, rate_translation);
    actual.rotation = low_pass(actual.rotation, target.rotation, rate_rotation);
    actual.zoom = low_pass(actual.zoom, target.zoom, rate_translation);
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

fn panic_on_ctrl_d(input: &InputState) {
    if input.is_key_pressed(Key::ControlLeft) && input.is_key_pressed(Key::KeyD) {
        info!("Exiting.");
        panic!();
    }
}

fn ping_on_c(
    particles: &mut Vec<RingParticle>,
    events: &VecDeque<Event>,
    camera: &Camera2D,
    mouse_screen_position: Option<Vec2>,
    transactions: &mut Vec<Action>,
) {
    let Some(screen_pos) = mouse_screen_position else {
        return;
    };

    let pos = screen_to_world(camera, screen_pos);

    let pressed_c = events
        .iter()
        .any(|e| e.event_type == rdev::EventType::KeyPress(Key::KeyC));

    if pressed_c {
        let particle = RingParticle::new(pos);
        particles.push(particle);
        transactions.push(Action::Ping(pos));
    }
}

fn reset_camera_on_ctrl_r(input: &InputState, target: &mut Camera2D) {
    if input.is_key_pressed(Key::ControlLeft) && input.is_key_pressed(Key::KeyR) {
        debug!("Reset camera");
        target.target = Vector2::zero();
        target.rotation = 0.0;
        target.zoom = 8.0;
    }
}

fn toggle_camera_local_normal_snapping(events: &VecDeque<Event>, snap_camera: &mut bool) {
    for e in events {
        if let rdev::EventType::KeyPress(Key::KeyT) = e.event_type {
            debug!("Toggle snap camera");
            *snap_camera ^= true;
        }
    }
}

fn toggle_following_on_key_f(
    events: &VecDeque<Event>,
    sel: &SelectionInfo,
    follow: &mut Option<Ent>,
) {
    let pressed_f = events
        .iter()
        .any(|e| e.event_type == rdev::EventType::KeyPress(Key::KeyF));

    if !pressed_f {
        return;
    }

    let Some((id, _delta)) = sel.mouse_hovered else {
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
        let accel = grid.linear_acceleration();
        grid.isometry.translation += grid.linear_velocity * dt;
        grid.linear_velocity += accel * dt;
        grid.isometry.rotation += grid.angular_velocity * dt;
    }
}

fn update_thrusters(thrusters: &mut Components<Thruster>) -> BTreeSet<Ent> {
    let mut grids_to_update = BTreeSet::new();
    for t in thrusters.values_mut() {
        if t.is_on && chance(0.005) {
            t.is_on = false;
            grids_to_update.insert(t.grid_id);
        } else if !t.is_on && chance(0.001) {
            t.is_on = true;
            grids_to_update.insert(t.grid_id);
        }
    }
    grids_to_update
}

fn update_selection_info(
    info: &mut SelectionInfo,
    grids: &Components<VehicleGrid>,
    camera: &Camera2D,
    mouse_screen_position: Option<Vec2>,
) {
    let pos = camera.target;
    let test_pos = raylib_to_glam_invert_y(pos);
    info.camera_hovered = find::closest_grid(grids, test_pos);
    if let Some(pos) = mouse_screen_position {
        let pos = screen_to_world(camera, pos);
        info.mouse_hovered = find::closest_grid(grids, pos);
    } else {
        info.mouse_hovered = None;
    }
}

fn set_target_camera_if_following(
    follow: Option<Ent>,
    grids: &Components<VehicleGrid>,
    target: &mut Camera2D,
) {
    let Some(follow) = follow else {
        return;
    };

    let Some(grid) = grids.get(follow) else {
        return;
    };

    target.target = glam_to_raylib_swap_y(grid.isometry.translation);
    target.rotation = grid.isometry.rotation.to_degrees();
}

pub fn update_world(
    world: &mut World,
    screen_dims: Vec2,
    mouse_screen_position: Option<Vec2>,
) -> Vec<Action> {
    world.ticks += 1;

    let mut outgoing_messages = Vec::new();

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
    // update_lights(&mut world.lights);
    update_computers(&mut world.computers);
    // world.grids_to_update = update_thrusters(&mut world.thrusters);

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

    set_target_camera_if_following(world.follow_vehicle, &world.grids, &mut world.target_camera);

    if world.snap_camera_to_local_planet {
        snap_camera_target_to_local_up(&mut world.target_camera);
    }

    reset_camera_on_ctrl_r(&world.input, &mut world.target_camera);
    update_camera(&world.target_camera, &mut world.camera);

    // spawn_random_ring_effects(&mut world.particles);

    panic_on_ctrl_d(&world.input);
    ping_on_c(
        &mut world.particles,
        &world.event_queue,
        &world.camera,
        world.mouse_screen_position,
        &mut outgoing_messages,
    );

    world.event_queue.clear();

    let end = std::time::Instant::now();

    world.timers.update = end - start;

    outgoing_messages
}

fn update_ring_particles(particles: &mut Vec<RingParticle>) {
    for ring in particles.iter_mut() {
        ring.step()
    }
    particles.retain(|p| p.is_alive());
}

fn update_lights(lights: &mut Components<Light>) {
    for light in lights.values_mut() {
        light.ticks += 1;
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

pub fn push_event(world: &mut World, event: Event) {
    world.event_queue.push_back(event);
}
