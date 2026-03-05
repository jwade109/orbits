use crate::camera::Camera;
use crate::chat::Chat;
use crate::components::*;
use crate::computer::*;
use crate::input_state::*;
use crate::light::*;
use crate::multiplayer::Action;
use crate::part::*;
use crate::persistence::save_world;
use crate::result::BaryError;
use crate::result::BaryResult;
use crate::ring_particle::PingParticle;
use crate::sounds::*;
use crate::systems::*;
use crate::thruster::*;
use crate::utils::*;
use crate::vehicle_grid::*;
use bary_core::prelude::PI;
use bary_core::prelude::*;
use log::*;
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
    pub physics: Duration,
    pub input: Duration,
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
    pub fira_code: MaybeFont,
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
    assets.lato_regular = rl
        .load_font_ex(thread, "assets/fonts/Lato-Regular.ttf", 48, None)
        .ok();
    assets.fira_code = rl
        .load_font_ex(thread, "assets/fonts/FiraCode-Bold.ttf", 128, None)
        .ok();

    // for (proto, tex) in assets.part_textures.values_mut() {
    //     let filename = format!("assets/parts/{}/skin.png", proto.part_name());
    //     *tex = rl.load_texture(thread, &filename).ok();
    // }
}

fn update_camera_target(
    input: &InputState,
    target: &mut Camera,
    follow: &mut Option<Ent>,
    sounds: &mut SoundEffects,
) {
    let angular_speed = 2.5f32.to_radians();
    let speed = 40.0 / target.zoom;
    let zoom_scale = 1.07;

    let old_follow = *follow;

    let right = rotate(Vec2::X, target.isometry.rotation);
    let up = rotate(right, PI / 2.0);

    if input.is_key_pressed(Key::ControlLeft) {
        return;
    }

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

    if old_follow.is_some() && follow.is_none() {
        sounds.push(SoundEffect::LeaveFollow);
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

// TODO(testing) very important to test!
pub fn remove_part(world: &mut World, part_id: Ent) -> BaryResult<()> {
    debug!("Removing part {}", part_id);
    let part = world.parts.try_get(part_id)?;
    let grid_id = part.grid_id;
    let proto = world.prototypes.try_get(part.prototype)?;
    let grid = world.grids.try_get_mut(grid_id)?;
    let name = grid.name.clone();

    if grid.thrusters.contains(&part_id) {
        world.thrusters.despawn(part_id)?;
    }
    if grid.computers.contains(&part_id) {
        world.computers.despawn(part_id)?;
    }
    if grid.lights.contains(&part_id) {
        world.lights.despawn(part_id)?;
    }

    grid.remove_from_index(part_id);
    grid.parts_mass -= proto.mass;

    world.parts.despawn(part_id)?;

    grid.assess_integrity();

    if grid.parts.is_empty() {
        world.grids.despawn(grid_id)?;
        world.chat.log(format!("Deleted empty grid \"{}\"", name));
    }

    Ok(())
}

pub fn remove_top_part_at(world: &mut World, grid_id: Ent, coord: PartCoord) -> BaryResult<Ent> {
    warn!("Destroying top part at {} in grid {}", coord, grid_id);

    let grid = world.grids.try_get(grid_id)?;
    let top_part = grid
        .get_parts_at(coord)
        .map(|occ| occ.top())
        .flatten()
        .ok_or(BaryError::NoPartsAt(coord))?;

    debug!("Top part is {}", top_part);

    remove_part(world, top_part)?;

    Ok(top_part)
}

pub mod input_handlers {

    use super::*;

    pub fn command_selected_ship_to_waypoint(world: &mut World, sounds: &mut SoundEffects) {
        let Some(grid_id) = world.selection_info.selected_grid else {
            return;
        };

        let Some(screen_pos) = world.mouse_screen_position else {
            return;
        };

        let world_pos = screen_to_world(&world.camera, screen_pos, world.screen_dims);

        let waypoint = Isometry2d::new(world_pos, 0.0);

        if let Err(e) = world::set_primary_computer_waypoint(grid_id, waypoint, world) {
            error!("Failed to set waypoint: {e:?}");
        }
        if let Err(e) = world::set_primary_computer_state(grid_id, true, world) {
            error!("Failed to turn primary computer on: {e:?}");
        }

        sounds.push(SoundEffect::SetWaypoint);
    }

    pub fn destroy_top_layer_part_at_mouseover(world: &mut World, sounds: &mut SoundEffects) {
        let Some(grid_id) = world.selection_info.selected_grid else {
            return;
        };

        let Some((coord, _occ)) = world.selection_info.mouseover_part_info else {
            return;
        };

        match remove_top_part_at(world, grid_id, coord) {
            Ok(id) => {
                info!("Removed part {}", id);
                sounds.push(SoundEffect::DestroyPart);
            }
            Err(e) => {
                error!("Failed to remove: {:?}", e);
            }
        }
    }

    pub fn apply_scroll_wheel_to_camera_target(delta_y: i64, target: &mut Camera) {
        let scale = 1.15;
        if delta_y > 0 {
            target.zoom *= scale;
        } else if delta_y < 0 {
            target.zoom /= scale;
        }
    }

    pub fn panic_on_ctrl_d(input: &InputState) {
        if input.is_key_pressed(Key::ControlLeft) {
            info!("Exiting.");
            panic!();
        }
    }

    pub fn save_on_ctrl_s(world: &mut World, input: &InputState) {
        let pressed_ctrl = input.is_key_pressed(Key::ControlLeft);

        if !pressed_ctrl {
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

    pub fn toggle_following_on_key_f(world: &mut World, sounds: &mut SoundEffects) {
        let Some(grid_id) = world.selection_info.selected_grid else {
            return;
        };
        debug!("Following {}", grid_id);
        world.follow_vehicle = Some(grid_id);
        sounds.push(SoundEffect::Follow);
    }

    pub fn ping_on_alt_left_click(
        world: &mut World,
        input: &InputState,
        actions: &mut Vec<Action>,
        sounds: &mut SoundEffects,
    ) {
        let Some(screen_pos) = world.mouse_screen_position else {
            return;
        };

        if !input.is_key_pressed(Key::Alt) {
            return;
        }

        let pos = screen_to_world(&world.camera, screen_pos, world.screen_dims);

        let particle = PingParticle::new(pos);
        world.particles.push(particle);
        actions.push(Action::Ping(pos));
        world.chat.log(format!("Pinged {}", pos));
        sounds.push(SoundEffect::Ping);
    }

    pub fn toggle_camera_local_normal_snapping_on_t(world: &mut World) {
        world.snap_camera_to_local_planet ^= true;
    }

    pub fn reset_camera_on_ctrl_r(world: &mut World, input: &InputState) {
        if input.is_key_pressed(Key::ControlLeft) {
            debug!("Reset camera");
            world.target_camera.isometry.translation = Vec2::ZERO;
            world.target_camera.isometry.rotation = 0.0;
            world.target_camera.zoom = 8.0;
        }
    }

    pub fn spawn_random_ship_on_p(world: &mut World) {
        if let Ok(grid_id) = world::spawn_grid_by_name(world, "remora") {
            let pos = randvec(10.0, 200.0);
            _ = world::set_grid_isometry(world, grid_id, Isometry2d::from_pos(pos));
        }
    }
}

fn is_key_just_pressed(events: &Vec<Event>, key: Key) -> bool {
    events
        .iter()
        .any(|e| e.event_type == rdev::EventType::KeyPress(key))
}

fn is_button_just_pressed(events: &Vec<Event>, button: Button) -> bool {
    events
        .iter()
        .any(|e| e.event_type == rdev::EventType::ButtonPress(button))
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

fn update_thrusters(
    thrusters: &mut Components<Thruster>,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    computers: &Components<Computer>,
) -> BTreeSet<Ent> {
    let mut needs_update = BTreeSet::new();

    for (grid_id, grid) in grids.iter() {
        let Some(cpu_id) = grid.computers.first() else {
            continue;
        };
        let Ok(cpu) = computers.try_get(*cpu_id) else {
            continue;
        };
        if !cpu.fired_this_tick {
            continue;
        }

        for thruster_id in &grid.thrusters {
            let Ok(thruster) = thrusters.try_get_mut(*thruster_id) else {
                continue;
            };
            let Ok(part) = parts.try_get(*thruster_id) else {
                continue;
            };

            let ctrl = cpu.vehicle_control;

            let tac = match part.placement.rot() {
                Rotation::East => ctrl.plus_x,
                Rotation::North => ctrl.neg_y,
                Rotation::West => ctrl.neg_x,
                Rotation::South => ctrl.plus_y,
            };

            // TODO(optimization) reduce lookups by storing isometry on the thruster?
            let isometry = part.placement.center_isometry();
            let center_of_thrust = isometry.translation;
            let rotation = part.placement.rot();
            let wrench = body_frame_wrench(thruster.thrust, center_of_thrust, rotation, Vec2::ZERO);

            if thruster.is_rcs {
                let can_torque = wrench.rotation.abs() > 0.5 && ctrl.attitude.abs() > 0.5;
                let is_torque =
                    can_torque && wrench.rotation.signum() as f64 == ctrl.attitude.signum();
                let is_linear = tac.throttle > 0.0 && tac.use_rcs;
                thruster.is_on = is_linear || is_torque;
            } else {
                thruster.is_on = !tac.use_rcs && tac.throttle > 0.0;
            }

            thruster.last_controlled_by = Some(*cpu_id);
        }

        needs_update.insert(*grid_id);
    }

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
    sounds: &mut SoundEffects,
) {
    let old_parts = sel.mouseover_part_info.map(|(_, occ)| occ);

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

    let new_parts = sel.mouseover_part_info.map(|(_, occ)| occ);
    let has_new_parts = new_parts.map(|e| e.has_any()).unwrap_or(false);

    if old_parts != new_parts && has_new_parts {
        sounds.push(SoundEffect::MouseoverPart);
    }
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

fn select_hovered_vehicle_on_click(sel: &mut SelectionInfo, sounds: &mut SoundEffects) {
    let old_grid = sel.selected_grid;

    sel.selected_grid = sel.mouse_hovered;

    if old_grid.is_some() {
        sel.selected_part_info = sel.mouseover_part_info;
    }

    if sel.selected_grid.is_some() {
        sounds.push(SoundEffect::Open);
    } else if old_grid.is_some() {
        sounds.push(SoundEffect::Close);
    }

    info!("Selected {:?}", sel.selected_grid);
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
                computer.pose.translation = randvec(100.0, 5000.0);
                computer.pose.rotation = rand(0.1, 0.7);
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
    // let mut input = InputState::default();
    // update_world_silly(world, &mut input, vec![])
    do_internal_world_physics(world);
    (vec![], SoundEffects::default())
}

pub fn do_internal_world_physics(world: &mut World) {
    let start = std::time::Instant::now();
    world.ticks += 1;
    update_ring_particles(&mut world.particles);
    let dirty_set = update_thrusters(
        &mut world.thrusters,
        &world.grids,
        &world.parts,
        &world.computers,
    );
    update_grid_acceleration(dirty_set, &mut world.grids, &world.thrusters, &world.parts);
    update_computers(&mut world.computers, &world.grids);
    propagate_grid_rigid_bodies(&mut world.grids);

    set_target_camera_if_following(
        world.follow_vehicle,
        &world.grids,
        &mut world.target_camera,
        &mut world.camera,
    );

    if world.snap_camera_to_local_planet {
        snap_camera_target_to_local_up(&mut world.target_camera);
    }

    update_camera(&world.target_camera, &mut world.camera);

    world.chat.drop_old_messages();

    let end = std::time::Instant::now();

    world.timers.physics = end - start;
}

pub fn process_event(
    world: &mut World,
    input: &mut InputState,
    event: &rdev::Event,
) -> (SoundEffects, Vec<Action>) {
    let mut actions = Vec::new();
    let mut sounds = SoundEffects::new();

    if let rdev::EventType::KeyPress(k) = event.event_type {
        input.set_pressed(k);
    } else if let rdev::EventType::KeyRelease(k) = event.event_type {
        input.set_released(k);
    }

    match event.event_type {
        rdev::EventType::KeyPress(key) => match key {
            Key::KeyS => input_handlers::save_on_ctrl_s(world, input),
            Key::KeyF => input_handlers::toggle_following_on_key_f(world, &mut sounds),
            Key::KeyT => input_handlers::toggle_camera_local_normal_snapping_on_t(world),
            Key::KeyR => input_handlers::reset_camera_on_ctrl_r(world, input),
            Key::KeyD => input_handlers::panic_on_ctrl_d(input),
            Key::KeyP => input_handlers::spawn_random_ship_on_p(world),
            _ => (),
        },
        rdev::EventType::KeyRelease(_key) => (),
        rdev::EventType::ButtonPress(button) => match button {
            Button::Left => {
                input_handlers::ping_on_alt_left_click(world, input, &mut actions, &mut sounds);
                select_hovered_vehicle_on_click(&mut world.selection_info, &mut sounds);
            }
            Button::Right => {
                input_handlers::destroy_top_layer_part_at_mouseover(world, &mut sounds)
            }
            Button::Middle => input_handlers::command_selected_ship_to_waypoint(world, &mut sounds),
            Button::Unknown(_) => (),
        },
        rdev::EventType::ButtonRelease(_button) => (),
        rdev::EventType::MouseMove { x: _, y: _ } => (),
        rdev::EventType::Wheel {
            delta_x: _,
            delta_y,
        } => {
            input_handlers::apply_scroll_wheel_to_camera_target(delta_y, &mut world.target_camera);
        }
    }

    (sounds, actions)
}

#[deprecated]
pub fn update_world_silly(world: &mut World, input: &InputState) -> (Vec<Action>, SoundEffects) {
    do_internal_world_physics(world);

    let input_start = std::time::Instant::now();

    let mut sounds = SoundEffects::default();

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
        &mut sounds,
    );

    update_camera_target(
        &input,
        &mut world.target_camera,
        &mut world.follow_vehicle,
        &mut sounds,
    );

    let end = std::time::Instant::now();
    world.timers.input = end - input_start;

    (Vec::new(), sounds)
}

fn update_ring_particles(particles: &mut Vec<PingParticle>) {
    for ring in particles.iter_mut() {
        ring.step()
    }
    particles.retain(|p| p.is_alive());
}
