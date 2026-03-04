use crate::camera::Camera;
use crate::chat::Chat;
use crate::components::*;
use crate::computer::*;
use crate::input_state::*;
use crate::light::*;
use crate::multiplayer::Action;
use crate::part::*;
use crate::persistence::save_world;
use crate::ring_particle::PingParticle;
use crate::sounds::*;
use crate::systems::*;
use crate::thruster::*;
use crate::utils::*;
use crate::vehicle_grid::*;
use bary_core::prelude::PI;
use bary_core::prelude::*;
use log::{debug, info};
use raylib::prelude::*;
use rdev::Button;
use rdev::Event;
use serde::{Deserialize, Serialize};
use std::collections::*;
use std::time::Duration;

pub type MaybeTexture = Option<Texture2D>;

pub type MaybeFont = Option<Font>;

#[derive(Default, Deserialize, Serialize, Clone)]
pub struct Timers {
    pub update: Duration,
    pub render: Duration,
    pub total: Duration,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct SelectionInfo {
    pub camera_hovered: Option<Ent>,
    pub mouse_hovered: Option<Ent>,
    pub selected_grid: Option<Ent>,
    pub mouseover_part_info: Option<(PartCoord, PartOccupancy)>,
    pub selected_part_info: Option<(PartCoord, PartOccupancy)>,
}

#[derive(Default)]
pub struct Assets {
    pub circle_texture: MaybeTexture,
    pub lato_regular: MaybeFont,
    pub part_textures: BTreeMap<String, Texture2D>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct World {
    pub ticks: u64,
    pub timers: Timers,
    #[serde(skip)]
    pub chat: Chat,
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
    pub camera: Camera,
    pub target_camera: Camera,
    pub particles: Vec<PingParticle>,
    pub blueprints: Components<NamedBlueprint>,
    pub prototypes: Components<PartPrototype>,
    pub parts: Components<Part>,
    pub thrusters: Components<Thruster>,
    pub computers: Components<Computer>,
    pub lights: Components<Light>,
    pub grids: Components<VehicleGrid>,
    pub sounds: SoundEffects,
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
            chat: Chat::default(),
            mouse_screen_position: None,
            selection_info: SelectionInfo::default(),
            spawner: EntitySpawner::default(),
            snap_camera_to_local_planet: false,
            follow_vehicle: None,
            screen_dims: Vec2::new(1500.0, 900.0),
            input: InputState::default(),
            event_queue: VecDeque::new(),
            camera: Camera {
                zoom: 0.1,
                ..Camera::default()
            },
            target_camera: Camera {
                zoom: 8.0,
                ..Camera::default()
            },
            particles: Vec::default(),
            blueprints: Components::default(),
            prototypes: Components::default(),
            parts: Components::default(),
            grids: Components::default(),
            thrusters: Components::default(),
            computers: Components::default(),
            lights: Components::default(),
            sounds: SoundEffects::default(),
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

fn update_camera_target(input: &InputState, target: &mut Camera, follow: &mut Option<Ent>) {
    let angular_speed = 2.5f32.to_radians();
    let speed = 40.0 / target.zoom;
    let zoom_scale = 1.07;

    let right = rotate(Vec2::X, target.isometry.rotation);
    let up = rotate(right, PI / 2.0);

    if input.is_key_pressed(Key::Minus) {
        target.zoom /= zoom_scale;
    }
    if input.is_key_pressed(Key::Equal) {
        target.zoom *= zoom_scale;
    }
    if input.is_key_pressed(Key::KeyQ) {
        target.isometry.rotation += angular_speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyE) {
        target.isometry.rotation -= angular_speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyS) {
        target.isometry.translation -= up * speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyW) {
        target.isometry.translation += up * speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyD) {
        target.isometry.translation += right * speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyA) {
        target.isometry.translation -= right * speed;
        *follow = None;
    }
}

fn update_camera(target: &Camera, actual: &mut Camera) {
    let rate_translation = 0.2;
    let rate_rotation = 0.2;
    actual.isometry.translation.x = low_pass(
        actual.isometry.translation.x,
        target.isometry.translation.x,
        rate_translation,
    );
    actual.isometry.translation.y = low_pass(
        actual.isometry.translation.y,
        target.isometry.translation.y,
        rate_translation,
    );
    actual.isometry.rotation = low_pass(
        actual.isometry.rotation,
        target.isometry.rotation,
        rate_rotation,
    );
    actual.zoom = low_pass(actual.zoom, target.zoom, rate_translation);
}

fn apply_scroll_wheel_to_camera_target(events: &VecDeque<Event>, target: &mut Camera) {
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

fn save_on_ctrl_s(world: &mut World) {
    let pressed_ctrl = world.input.is_key_pressed(Key::ControlLeft);
    let pressed_s = is_key_just_pressed(&world.event_queue, Key::KeyS);

    if !(pressed_ctrl && pressed_s) {
        return;
    }

    let now = chrono::offset::Local::now();
    let filename = format!("./saves/world-{}/", now.format("%Y-%m-%d-%H-%M-%S"));
    match save_world(&filename, world, true) {
        Ok(_) => {
            let s = format!("Saved to {}", filename);
            world.chat.log(s);
        }
        Err(e) => {
            let s = format!("Failed to save: {:?}", e);
            world.chat.log(s);
        }
    }
}

fn ping_on_alt_left_click(
    particles: &mut Vec<PingParticle>,
    input: &InputState,
    events: &VecDeque<Event>,
    camera: &Camera,
    mouse_screen_position: Option<Vec2>,
    transactions: &mut Vec<Action>,
    screen_dims: Vec2,
    chat: &mut Chat,
    sounds: &mut SoundEffects,
) {
    let Some(screen_pos) = mouse_screen_position else {
        return;
    };

    let pos = screen_to_world(camera, screen_pos, screen_dims);

    // TODO(cleanup) use a function here
    let left_click = events
        .iter()
        .any(|e| e.event_type == rdev::EventType::ButtonPress(Button::Left));

    let alt = input.is_key_pressed(Key::Alt);

    if left_click && alt {
        let particle = PingParticle::new(pos);
        particles.push(particle);
        transactions.push(Action::Ping(pos));
        chat.log(format!("Pinged {}", pos));
        sounds.effects.push(SoundEffect::Close);
    }
}

fn is_key_just_pressed(events: &VecDeque<Event>, key: Key) -> bool {
    events
        .iter()
        .any(|e| e.event_type == rdev::EventType::KeyPress(key))
}

fn is_button_just_pressed(events: &VecDeque<Event>, button: Button) -> bool {
    events
        .iter()
        .any(|e| e.event_type == rdev::EventType::ButtonPress(button))
}

fn spawn_random_ship_on_p(world: &mut World) {
    if is_key_just_pressed(&world.event_queue, Key::KeyP) {
        if let Ok(grid_id) = world::spawn_grid_by_name(world, "remora") {
            let pos = randvec(10.0, 200.0);
            _ = world::set_grid_isometry(world, grid_id, Isometry2d::from_pos(pos));
        }
    }
}

fn reset_camera_on_ctrl_r(input: &InputState, target: &mut Camera) {
    if input.is_key_pressed(Key::ControlLeft) && input.is_key_pressed(Key::KeyR) {
        debug!("Reset camera");
        target.isometry.translation = Vec2::ZERO;
        target.isometry.rotation = 0.0;
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

    let Some(grid_id) = sel.selected_grid else {
        return;
    };

    *follow = Some(grid_id);
}

fn snap_camera_target_to_local_up(target: &mut Camera) {
    let r = 100.0;
    let q = if target.isometry.translation.length() < r {
        target.isometry.translation.normalize_or_zero() * r
    } else {
        target.isometry.translation
    };

    target.isometry.rotation = q.to_angle() + PI / 2.0;
    target.isometry.translation = q;
}

fn propagate_grid_rigid_bodies(grids: &mut Components<VehicleGrid>) {
    let dt = 0.02;
    for grid in grids.values_mut() {
        let body_frame_accel = grid.linear_acceleration();
        let omega = grid.angular_acceleration();
        let accel = rotate(body_frame_accel, grid.pose.rotation);
        grid.pose.translation += grid.velocity.translation * dt;
        grid.velocity.translation += accel * dt;
        grid.pose.rotation += grid.velocity.rotation * dt;
        grid.velocity.rotation += omega * dt;
    }
}

fn update_thrusters(thrusters: &mut Components<Thruster>) -> BTreeSet<Ent> {
    let mut needs_update = BTreeSet::new();
    // for t in thrusters.values_mut() {
    //     if t.is_on && chance(0.02) {
    //         t.is_on = false;
    //         needs_update.insert(t.grid_id);
    //     } else if !t.is_on && chance(0.001) {
    //         t.is_on = true;
    //         needs_update.insert(t.grid_id);
    //     }
    // }
    needs_update
}

fn update_selection_info(
    info: &mut SelectionInfo,
    grids: &Components<VehicleGrid>,
    camera: &Camera,
    mouse_screen_position: Option<Vec2>,
    screen_dims: Vec2,
) {
    info.camera_hovered =
        find::closest_grid(grids, camera.isometry.translation, 100.0).map(|e| e.0);
    if let Some(pos) = mouse_screen_position {
        let pos = screen_to_world(camera, pos, screen_dims);
        info.mouse_hovered = find::closest_grid(grids, pos, 100.0).map(|e| e.0);
    } else {
        info.mouse_hovered = None;
    }
}

fn update_mouseover_part_info(
    sel: &mut SelectionInfo,
    grids: &Components<VehicleGrid>,
    mouse_screen_position: Option<Vec2>,
    screen_dims: Vec2,
    camera: &Camera,
) {
    sel.mouseover_part_info = None;

    let Some(screen_pos) = mouse_screen_position else {
        return;
    };
    let Some(grid_id) = sel.selected_grid else {
        return;
    };
    let Ok(grid) = grids.try_get(grid_id) else {
        return;
    };

    let world_pos = screen_to_world(camera, screen_pos, screen_dims);

    let coord = PartCoord::from_meters_floored(in_frame(grid.pose, world_pos));

    let occ = grid.get_parts_at(coord).unwrap_or(&PartOccupancy::EMPTY);

    sel.mouseover_part_info = Some((coord, *occ));
}

fn set_target_camera_if_following(
    follow: Option<Ent>,
    grids: &Components<VehicleGrid>,
    target: &mut Camera,
    actual: &mut Camera,
) {
    let Some(follow) = follow else {
        return;
    };

    let Some(grid) = grids.get(follow) else {
        return;
    };

    target.isometry.translation = grid.pose.translation;
    target.isometry.rotation = grid.pose.rotation;

    actual.isometry.translation = target.isometry.translation;
    actual.isometry.rotation = target.isometry.rotation;
}

fn select_hovered_vehicle_on_click(events: &VecDeque<Event>, sel: &mut SelectionInfo) {
    let is_clicked = is_button_just_pressed(events, Button::Left);

    if !is_clicked {
        return;
    }

    let old_grid = sel.selected_grid;

    sel.selected_grid = sel.mouse_hovered;

    if old_grid.is_some() {
        sel.selected_part_info = sel.mouseover_part_info;
    }

    info!("Selected {:?}", sel.selected_grid)
}

pub fn update_computers(computers: &mut Components<Computer>, grids: &Components<VehicleGrid>) {
    for computer in computers.values_mut() {
        let Ok(grid) = grids.try_get(computer.grid_id) else {
            continue;
        };

        computer.status = match computer.on {
            true => MachineStatus::Running,
            false => MachineStatus::Off,
        };
        if computer.on {
            computer.ticks_this_cycle += 1;
            computer.fired_this_tick = computer.ticks_this_cycle == computer.ticks_per_cycle;
            if computer.fired_this_tick {
                computer.ticks_this_cycle = 0;
                computer.iters += 1;
            }
        } else {
            computer.fired_this_tick = false;
        }

        if computer.fired_this_tick {
            if computer.pose.translation == Vec2::ZERO {
                computer.pose.translation = randvec(100.0, 300.0);
                computer.pose.rotation = rand(0.1, 0.7);
            } else {
                computer.pose.translation += randvec(0.0, 2.0);
                computer.pose.rotation += rand(-0.05, 0.05);
            }

            let actual = PV::from_f64(grid.pose.translation, grid.velocity.translation);
            let target = PV::from_f64(computer.pose.translation, computer.velocity.translation);
            let body = RigidBody {
                pv: actual,
                angle: grid.pose.rotation as f64,
                angular_velocity: grid.velocity.rotation as f64,
            };
            let (ctrl, status) = position_hold_control_law(
                target,
                computer.pose.rotation as f64,
                &body,
                DVec2::ZERO,
            );

            computer.vehicle_control = ctrl;
            computer.control_status = status;
        }
    }
}

pub fn update_world(world: &mut World) -> (Vec<Action>, SoundEffects) {
    world.ticks += 1;

    world.sounds.effects.clear();

    let mut outgoing_messages = Vec::new();

    let start = std::time::Instant::now();

    toggle_camera_local_normal_snapping(&world.event_queue, &mut world.snap_camera_to_local_planet);
    toggle_following_on_key_f(
        &world.event_queue,
        &world.selection_info,
        &mut world.follow_vehicle,
    );

    select_hovered_vehicle_on_click(&world.event_queue, &mut world.selection_info);

    update_ring_particles(&mut world.particles);
    update_lights(&mut world.lights);
    update_computers(&mut world.computers, &world.grids);
    let dirty_set = update_thrusters(&mut world.thrusters);
    update_grid_acceleration(dirty_set, &mut world.grids, &world.thrusters, &world.parts);

    update_selection_info(
        &mut world.selection_info,
        &world.grids,
        &world.camera,
        world.mouse_screen_position,
        world.screen_dims,
    );

    update_mouseover_part_info(
        &mut world.selection_info,
        &world.grids,
        world.mouse_screen_position,
        world.screen_dims,
        &world.camera,
    );

    update_input_state(&world.event_queue, &mut world.input);
    apply_scroll_wheel_to_camera_target(&world.event_queue, &mut world.target_camera);

    propagate_grid_rigid_bodies(&mut world.grids);

    update_camera_target(
        &world.input,
        &mut world.target_camera,
        &mut world.follow_vehicle,
    );

    set_target_camera_if_following(
        world.follow_vehicle,
        &world.grids,
        &mut world.target_camera,
        &mut world.camera,
    );

    if world.snap_camera_to_local_planet {
        snap_camera_target_to_local_up(&mut world.target_camera);
    }

    reset_camera_on_ctrl_r(&world.input, &mut world.target_camera);
    update_camera(&world.target_camera, &mut world.camera);

    // spawn_random_ring_effects(&mut world.particles);

    panic_on_ctrl_d(&world.input);

    save_on_ctrl_s(world);

    ping_on_alt_left_click(
        &mut world.particles,
        &world.input,
        &world.event_queue,
        &world.camera,
        world.mouse_screen_position,
        &mut outgoing_messages,
        world.screen_dims,
        &mut world.chat,
        &mut world.sounds,
    );

    world.chat.drop_old_messages();

    spawn_random_ship_on_p(world);

    world.event_queue.clear();

    let end = std::time::Instant::now();

    world.timers.update = end - start;

    let sounds = world.sounds.clone();
    world.sounds = SoundEffects::default();

    (outgoing_messages, sounds)
}

fn update_ring_particles(particles: &mut Vec<PingParticle>) {
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

pub fn push_event(world: &mut World, event: Event) {
    world.event_queue.push_back(event);
}
