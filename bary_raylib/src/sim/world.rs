use crate::camera::Camera;
use crate::client::*;
use crate::components::*;
use crate::input_state::*;
use crate::multiplayer::Action;
use crate::ops::destroy_part_without_integrity_check;
use crate::ops::detach_part_from_parent;
use crate::persistence::save_world;
use crate::query::closest_grid;
use crate::result::BaryError;
use crate::result::BaryResult;
use crate::sim::*;
use crate::sounds::*;
use crate::utils::*;
use bary_core::prelude::PI;
use bary_core::prelude::*;
use log::*;
use rdev::Button;
use serde::{Deserialize, Serialize};
use std::collections::*;
use std::time::Duration;

#[derive(Default, Deserialize, Serialize, Clone)]
pub struct Timers {
    pub physics: Duration,
    pub render: Duration,
    pub total: Duration,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct World {
    pub ticks: u64,
    pub tick_rate: u32,
    pub timers: Timers,

    // camera info
    pub camera: Camera,
    pub target_camera: Camera,

    // debug info
    pub grid_acceleration_updates: u64,

    // components - to be synchronized between clients
    pub spawner: EntitySpawner,
    pub particles: Vec<PingParticle>,
    pub blueprints: Components<NamedBlueprint>,
    pub prototypes: Components<PartPrototype>,
    pub parts: Components<Part>,
    pub thrusters: Components<Thruster>,
    pub computers: Components<Computer>,
    pub lights: Components<Light>,
    pub grids: Components<VehicleGrid>,
    pub tracking: Components<Tracker>,
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
            tick_rate: 2,
            timers: Timers::default(),
            spawner: EntitySpawner::default(),
            camera: Camera {
                zoom: 0.1,
                ..Camera::default()
            },
            target_camera: Camera {
                zoom: 8.0,
                ..Camera::default()
            },
            grid_acceleration_updates: 0,
            particles: Vec::default(),
            blueprints: Components::default(),
            prototypes: Components::default(),
            parts: Components::default(),
            grids: Components::default(),
            thrusters: Components::default(),
            computers: Components::default(),
            lights: Components::default(),
            tracking: Components::default(),
        }
    }
}

pub fn size_in_bytes(world: &World) -> usize {
    let bytes = bincode::serialize(world).unwrap();
    bytes.len()
}

fn camera_zooms_with_plus_minus(input: &InputState, target: &mut Camera) {
    let zoom_scale = 1.07;

    if input.is_key_pressed(Key::Minus) {
        target.zoom /= zoom_scale;
    }
    if input.is_key_pressed(Key::Equal) {
        target.zoom *= zoom_scale;
    }
}

fn camera_moves_with_wasd(
    input: &InputState,
    target: &mut Camera,
    follow: &mut Option<Ent>,
    sounds: &mut SoundEffects,
) {
    let angular_speed = 2.5f32.to_radians();
    let speed = 40.0 / target.zoom;

    let old_follow = *follow;

    let right = rotate(Vec2::X, target.isometry.rotation);
    let up = rotate(right, PI / 2.0);

    if input.is_key_pressed(Key::ControlLeft) {
        return;
    }

    if input.is_key_pressed(Key::KeyQ) {
        target.isometry.rotation += angular_speed;
        // *follow = None;
    }
    if input.is_key_pressed(Key::KeyE) {
        target.isometry.rotation -= angular_speed;
        // *follow = None;
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

fn animate_camera_towards_target(target: &Camera, actual: &mut Camera) {
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

pub fn destroy_part(world: &mut World, part_id: Ent) -> BaryResult<(PartInstance, Ent)> {
    let (instance, grid_id) = destroy_part_without_integrity_check(world, part_id, true)?;
    split_grid_if_necessary(world, grid_id)?;
    Ok((instance, grid_id))
}

pub fn destroy_top_part_at(
    world: &mut World,
    grid_id: Ent,
    coord: PartCoord,
) -> BaryResult<(PartInstance, Ent)> {
    warn!("Destroying top part at {} in grid {}", coord, grid_id);

    let grid = world.grids.try_get(grid_id)?;
    let top_part = grid
        .get_parts_at(coord)
        .map(|occ| occ.top())
        .flatten()
        .ok_or(BaryError::NoPartsAt(coord))?;

    debug!("Top part is {}", top_part);

    destroy_part(world, top_part)
}

pub fn detach_top_part_at(world: &mut World, grid_id: Ent, coord: PartCoord) -> BaryResult<Ent> {
    warn!("Detaching top part at {} in grid {}", coord, grid_id);

    let grid = world.grids.try_get(grid_id)?;
    let top_part = grid
        .get_parts_at(coord)
        .map(|occ| occ.top())
        .flatten()
        .ok_or(BaryError::NoPartsAt(coord))?;

    debug!("Top part is {}", top_part);

    detach_part_from_parent(world, top_part)?;

    Ok(top_part)
}

pub mod input_handlers {

    use crate::sim::systems::{
        update_grid_physical_props,
        world::{
            set_grid_pose, set_primary_computer_state, set_primary_computer_waypoint,
            spawn_grid_by_name, toggle_tracking,
        },
    };

    use super::*;

    pub fn command_selected_ship_to_waypoint(
        world: &mut World,
        client: &mut ClientSpecificInfo,
        sounds: &mut SoundEffects,
    ) {
        let Some(grid_id) = client.selection_info.selected_grid else {
            return;
        };

        let Some(screen_pos) = client.mouse_screen_position else {
            return;
        };

        let world_pos = screen_to_world(&world.camera, screen_pos, client.screen_dims);

        let waypoint = Isometry2d::new(world_pos, 0.0);

        if let Err(e) = set_primary_computer_waypoint(grid_id, waypoint, world) {
            client.chat.log(format!("Failed to set waypoint: {e:?}"));
            sounds.push(SoundEffect::GenericFailure);
            return;
        }

        if let Err(e) = set_primary_computer_state(grid_id, true, world) {
            client
                .chat
                .log(format!("Failed to turn primary computer on: {e:?}"));
            sounds.push(SoundEffect::GenericFailure);
            return;
        }

        sounds.push(SoundEffect::SetWaypoint);
    }

    pub fn destroy_top_layer_part_at_mouseover(
        world: &mut World,
        client: &mut ClientSpecificInfo,
        sounds: &mut SoundEffects,
    ) {
        let Some(grid_id) = client.selection_info.selected_grid else {
            return;
        };

        let Some((coord, _occ)) = client.selection_info.mouseover_part_info else {
            return;
        };

        match destroy_top_part_at(world, grid_id, coord) {
            Ok((instance, grid_id)) => {
                info!("Removed part {:?}, grid {}", instance, grid_id);
                sounds.push(SoundEffect::DestroyPart);
            }
            Err(_e) => {
                // don't care.
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

    pub fn save_on_ctrl_s(world: &mut World, client: &mut ClientSpecificInfo, input: &InputState) {
        let pressed_ctrl = input.is_key_pressed(Key::ControlLeft);

        if !pressed_ctrl {
            return;
        }

        let now = chrono::offset::Local::now();
        let filename = format!("./saves/world-{}/", now.format("%Y-%m-%d-%H-%M-%S"));
        match save_world(&filename, world, true) {
            Ok(_) => {
                let s = format!("Saved to {}", filename);
                client.chat.log(s);
            }
            Err(e) => {
                let s = format!("Failed to save: {:?}", e);
                client.chat.log(s);
            }
        }
    }

    pub fn toggle_following_on_key_f(client: &mut ClientSpecificInfo, sounds: &mut SoundEffects) {
        let Some(grid_id) = client.selection_info.selected_grid else {
            return;
        };
        if client.viewport.look_at(grid_id) {
            sounds.push(SoundEffect::Follow);
            debug!("Following {}", grid_id);
        }
    }

    pub fn ping_on_alt_left_click(
        world: &mut World,
        client: &mut ClientSpecificInfo,
        input: &InputState,
        actions: &mut Vec<Action>,
        sounds: &mut SoundEffects,
    ) {
        let Some(screen_pos) = client.mouse_screen_position else {
            return;
        };

        if !input.is_key_pressed(Key::Alt) {
            return;
        }

        let pos = screen_to_world(&world.camera, screen_pos, client.screen_dims);

        let particle = PingParticle::new(pos);
        world.particles.push(particle);
        actions.push(Action::Ping(pos));
        client.chat.log(format!("Pinged {}", pos));
        sounds.push(SoundEffect::Ping);
    }

    pub fn toggle_tracking_for_selected_grid(world: &mut World, client: &mut ClientSpecificInfo) {
        let Some(grid_id) = client.selection_info.selected_grid else {
            return;
        };
        match toggle_tracking(world, grid_id) {
            Ok(true) => client.chat.log(format!("Enabled tracking for {}", grid_id)),
            Ok(false) => client
                .chat
                .log(format!("Disabled tracking for {}", grid_id)),
            Err(e) => client
                .chat
                .log(format!("Failed to toggle tracking: {:?}", e)),
        }
    }

    pub fn reset_camera_on_ctrl_r(world: &mut World, input: &InputState) {
        if input.is_key_pressed(Key::ControlLeft) {
            debug!("Reset camera");
            world.target_camera.isometry.translation = Vec2::ZERO;
            world.target_camera.isometry.rotation = 0.0;
            world.target_camera.zoom = 8.0;
        }
    }

    pub fn lock_rotation_on_key_r(client: &mut ClientSpecificInfo, input: &InputState) {
        if input.is_key_pressed(Key::ControlLeft) {
            return;
        }
        if let Viewport::Free(fly) = &mut client.viewport {
            debug!("Toggle lock rotation");
            fly.lock_rotation ^= true;
        }
    }

    pub fn rotate_editor_part_on_key_r(client: &mut ClientSpecificInfo, input: &InputState) {
        if input.is_key_pressed(Key::ControlLeft) {
            return;
        }
        if let Viewport::Editor(editor) = &mut client.viewport {
            editor.part_rotation = editor.part_rotation.next();
            client.chat.log("Rotated");
        }
    }

    pub fn spawn_random_ship_on_p(world: &mut World) {
        if let Ok(grid_id) = spawn_grid_by_name(world, "remora") {
            let pos = randvec(10.0, 200.0);
            _ = set_grid_pose(world, grid_id, Isometry2d::from_pos(pos));
        }
    }

    pub fn update_center_of_mass_on_m(world: &mut World) {
        for grid in world.grids.values_mut() {
            _ = update_grid_physical_props(grid, &mut world.parts);
        }
    }

    pub fn leave_ship_editor_on_escape(
        world: &mut World,
        client: &mut ClientSpecificInfo,
        sounds: &mut SoundEffects,
    ) {
        let Viewport::Editor(editor) = &client.viewport else {
            return;
        };
        client.viewport = Viewport::Free(FreeFlying {
            follow_vehicle: Some(editor.vehicle),
            lock_rotation: false,
        });

        world.target_camera.zoom = 20.0;
        world.target_camera.isometry.rotation = 0.0;

        client.chat.log("Left ship editor");
        sounds.push(SoundEffect::LeaveEditor);
    }

    pub fn enter_ship_editor_on_enter(
        world: &mut World,
        client: &mut ClientSpecificInfo,
        sounds: &mut SoundEffects,
    ) {
        let Viewport::Free(_) = &client.viewport else {
            return;
        };

        let Some(id) = client.selection_info.selected_grid else {
            return;
        };

        client.viewport = Viewport::Editor(EditorState {
            vehicle: id,
            camera_offset: Vec2::ZERO,
            camera_rotation: Rotation::East,
            prototype_id: None,
            part_rotation: Rotation::East,
            layer: None,
        });

        world.target_camera.zoom = 40.0;

        client.chat.log("Switched to ship editor");
        sounds.push(SoundEffect::OpenEditor);
    }

    pub fn pipette_part_if_in_editor_on_q(world: &World, client: &mut ClientSpecificInfo) {
        let Viewport::Editor(editor) = &mut client.viewport else {
            return;
        };

        if editor.prototype_id.is_some() {
            editor.prototype_id = None;
            return;
        }

        editor.prototype_id = None;

        let Some(hovered_grid) = client.selection_info.mouse_hovered else {
            return;
        };

        if editor.vehicle != hovered_grid {
            return;
        }

        let Some((_part_coord, occ)) = client.selection_info.mouseover_part_info else {
            return;
        };

        let Some(part_id) = occ.top() else {
            return;
        };

        let Ok(part) = world.parts.try_get(part_id) else {
            return;
        };

        let Ok(proto) = world.prototypes.try_get(part.prototype) else {
            return;
        };

        editor.prototype_id = Some(part.prototype);
        editor.part_rotation = part.placement.rot();

        let s = format!("{:?} {:?}", editor.prototype_id, proto.name);
        client.chat.log(s);
    }
}

fn propagate_grid_rigid_bodies(grids: &mut Components<VehicleGrid>) {
    let dt = 0.02;
    for grid in grids.values_mut() {
        let body_frame_accel = grid.linear_acceleration();
        let omega = grid.angular_acceleration();
        let accel = rotate(body_frame_accel, grid.particle_location.rotation);
        grid.particle_location.translation += grid.velocity.translation * dt;
        grid.velocity.translation += accel * dt;
        grid.particle_location.rotation += grid.velocity.rotation * dt;
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
        if grid.thrusters.is_empty() {
            continue;
        }

        let Some(cpu_id) = grid.computers.first() else {
            continue;
        };
        let Ok(cpu) = computers.try_get(*cpu_id) else {
            continue;
        };
        if !cpu.fired_this_tick {
            continue;
        }

        let mut thruster_changed = false;

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
            let wrench = body_frame_wrench(
                thruster.thrust,
                center_of_thrust,
                rotation,
                grid.center_of_mass,
            );

            let old_val = thruster.is_on;

            if thruster.is_rcs {
                let can_torque = wrench.rotation.abs() > 0.5 && ctrl.attitude.abs() > 0.5;
                let is_torque =
                    can_torque && wrench.rotation.signum() as f64 == ctrl.attitude.signum();
                let is_linear = tac.throttle > 0.0 && tac.use_rcs;
                thruster.is_on = is_linear || is_torque;
            } else {
                thruster.is_on = !tac.use_rcs && tac.throttle > 0.0;
            }

            thruster_changed |= old_val != thruster.is_on;

            thruster.last_controlled_by = Some(*cpu_id);
        }

        if thruster_changed {
            needs_update.insert(*grid_id);
        }
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
    if let Some(pos) = mouse_screen_position {
        let pos = screen_to_world(camera, pos, screen_dims);
        info.mouse_hovered = super::systems::find::closest_grid(grids, pos, 100.0).map(|e| e.0);
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
    let origin = grid.origin();
    let coord = PartCoord::from_meters_floored(in_frame(origin, world_pos));

    let occ = grid.get_parts_at(coord).unwrap_or(&PartOccupancy::EMPTY);

    // if occ.has_any() {
    sel.mouseover_part_info = Some((coord, *occ));
    // }

    let new_parts = sel.mouseover_part_info.map(|(_, occ)| occ);
    let has_new_parts = new_parts.map(|e| e.has_any()).unwrap_or(false);

    if old_parts != new_parts && has_new_parts {
        sounds.push(SoundEffect::MouseoverPart);
    }
}

fn set_target_camera_if_following(
    follow: Option<Ent>,
    lock_rotation: bool,
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

    let iso = grid.centroid_isometry();

    target.isometry.translation = iso.translation;
    if lock_rotation {
        target.isometry.rotation = iso.rotation;
    }

    actual.isometry.translation = target.isometry.translation;
}

fn select_hovered_vehicle_on_click(sel: &mut SelectionInfo, sounds: &mut SoundEffects) {
    let old_grid = sel.selected_grid;

    sel.selected_grid = sel.mouse_hovered;

    if sel.selected_grid.is_some() {
        sounds.push(SoundEffect::Open);
    } else if old_grid.is_some() {
        sounds.push(SoundEffect::Close);
    }
}

fn editor_try_place_part_on_click(world: &mut World, client: &mut ClientSpecificInfo) {
    let Viewport::Editor(e) = &mut client.viewport else {
        return;
    };
    let Some((coord, occ)) = client.selection_info.mouseover_part_info else {
        return;
    };

    client
        .chat
        .log(format!("Clicked on the editor! {:?} {:?}", coord, occ));

    let Some(proto_id) = e.prototype_id else {
        return;
    };

    let Ok(proto) = world.prototypes.try_get(proto_id) else {
        return;
    };

    let placement = GridPlacement::new(coord, e.part_rotation, proto.dims);

    let instance = PartInstance {
        name: proto.name.clone(),
        layer: proto.layer,
        placement,
    };

    _ = insert_part(e.vehicle, world, &instance, true);
}

pub fn update_computers(
    computers: &mut Components<Computer>,
    parts: &Components<Part>,
    grids: &Components<VehicleGrid>,
    ticks: u64,
) {
    for (cpu_id, computer) in computers.iter_mut() {
        let Ok(part) = parts.try_get(*cpu_id) else {
            continue;
        };

        let command = if let Some(cmd) = computer.command_queue.first() {
            if cmd.ticks == ticks {
                info!("Doing command: {cmd:?}");
                let cmd = cmd.command.clone();
                computer.command_queue.remove(0);
                Some(cmd)
            } else {
                None
            }
        } else {
            None
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

        if let Some(command) = command {
            computer.vehicle_control = command;
            computer.control_status = VehicleControlStatus::ExecutingLaunchProgram;
        } else if computer.fired_this_tick && computer.command_queue.is_empty() {
            let Ok(grid) = grids.try_get(part.grid_id) else {
                continue;
            };
            let pose = grid.particle_location;

            let actual = PV::from_f64(pose.translation, grid.velocity.translation);
            let target = PV::from_f64(computer.pose.translation, computer.velocity.translation);
            let body = RigidBody {
                pv: actual,
                angle: pose.rotation as f64,
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

pub fn update_trackers(
    trackers: &mut Components<Tracker>,
    grids: &Components<VehicleGrid>,
    ticks: u64,
) {
    if ticks % 20 > 0 {
        return;
    }

    let mut to_despawn = BTreeSet::new();

    for (grid_id, tracker) in trackers.iter_mut() {
        let Ok(grid) = grids.try_get(*grid_id) else {
            to_despawn.insert(*grid_id);
            continue;
        };
        tracker.add(grid);
    }

    for id in to_despawn {
        _ = trackers.despawn(id);
    }
}

pub fn update_world(world: &mut World) {
    let start = std::time::Instant::now();
    world.ticks += 1;
    update_ring_particles(&mut world.particles);
    let dirty_set = update_thrusters(
        &mut world.thrusters,
        &world.grids,
        &world.parts,
        &world.computers,
    );
    world.grid_acceleration_updates += dirty_set.len() as u64;
    update_grid_acceleration(dirty_set, &mut world.grids, &world.thrusters, &world.parts);
    update_computers(
        &mut world.computers,
        &world.parts,
        &world.grids,
        world.ticks,
    );
    propagate_grid_rigid_bodies(&mut world.grids);
    update_trackers(&mut world.tracking, &world.grids, world.ticks);

    let end = std::time::Instant::now();

    world.timers.physics = end - start;
}

pub fn process_event(
    world: &mut World,
    client: &mut ClientSpecificInfo,
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
            Key::KeyS => input_handlers::save_on_ctrl_s(world, client, input),
            Key::KeyF => input_handlers::toggle_following_on_key_f(client, &mut sounds),
            Key::KeyT => input_handlers::toggle_tracking_for_selected_grid(world, client),
            Key::KeyR => {
                input_handlers::reset_camera_on_ctrl_r(world, input);
                input_handlers::lock_rotation_on_key_r(client, input);
                input_handlers::rotate_editor_part_on_key_r(client, input);
            }
            Key::KeyD => input_handlers::panic_on_ctrl_d(input),
            Key::KeyP => input_handlers::spawn_random_ship_on_p(world),
            Key::KeyM => input_handlers::update_center_of_mass_on_m(world),
            Key::KeyQ => input_handlers::pipette_part_if_in_editor_on_q(world, client),
            Key::Return => input_handlers::enter_ship_editor_on_enter(world, client, &mut sounds),
            Key::Escape => input_handlers::leave_ship_editor_on_escape(world, client, &mut sounds),
            _ => (),
        },
        rdev::EventType::KeyRelease(_key) => (),
        rdev::EventType::ButtonPress(button) => match button {
            Button::Left => {
                input_handlers::ping_on_alt_left_click(
                    world,
                    client,
                    input,
                    &mut actions,
                    &mut sounds,
                );
                select_hovered_vehicle_on_click(&mut client.selection_info, &mut sounds);
                editor_try_place_part_on_click(world, client);
            }
            Button::Right => {
                input_handlers::destroy_top_layer_part_at_mouseover(world, client, &mut sounds)
            }
            Button::Middle => {
                input_handlers::command_selected_ship_to_waypoint(world, client, &mut sounds)
            }
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

pub fn post_simulation_update(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    input: &InputState,
) -> (Vec<Action>, SoundEffects) {
    let mut sounds = SoundEffects::default();

    client.chat.drop_old_messages();

    update_selection_info(
        &mut client.selection_info,
        &world.grids,
        &world.camera,
        client.mouse_screen_position,
        client.screen_dims,
    );

    update_mouseover_part_info(
        &mut client.selection_info,
        &world.grids,
        client.mouse_screen_position,
        client.screen_dims,
        &world.camera,
        &mut sounds,
    );

    match &mut client.viewport {
        Viewport::Free(fly) => {
            set_target_camera_if_following(
                fly.follow_vehicle,
                fly.lock_rotation,
                &world.grids,
                &mut world.target_camera,
                &mut world.camera,
            );

            camera_moves_with_wasd(
                &input,
                &mut world.target_camera,
                &mut fly.follow_vehicle,
                &mut sounds,
            );

            camera_zooms_with_plus_minus(input, &mut world.target_camera);
        }
        Viewport::Editor(editor) => {
            camera_zooms_with_plus_minus(input, &mut world.target_camera);

            set_cams_to_grid_pose(
                editor.vehicle,
                &world.grids,
                &mut world.target_camera,
                &mut world.camera,
            );
        }
    }

    animate_camera_towards_target(&world.target_camera, &mut world.camera);

    (Vec::new(), sounds)
}

fn set_cams_to_grid_pose(
    grid_id: Ent,
    grids: &Components<VehicleGrid>,
    target: &mut Camera,
    actual: &mut Camera,
) {
    if let Ok(grid) = grids.try_get(grid_id) {
        target.isometry = grid.centroid_isometry();
        target.zoom = target.zoom.clamp(20.0, 150.0);
        actual.isometry = target.isometry;
    }
}

fn update_ring_particles(particles: &mut Vec<PingParticle>) {
    for ring in particles.iter_mut() {
        ring.step()
    }
    particles.retain(|p| p.is_alive());
}
