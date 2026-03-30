use crate::camera::Camera;
use crate::client::*;
use crate::components::*;
use crate::constants::*;
use crate::imgui::ZOOM_FAR_AWAY;
use crate::imgui::ZOOM_NEAR_VEHICLE;
use crate::input_state::*;
use crate::multiplayer::Action;
use crate::ops::destroy_part_without_integrity_check;
use crate::ops::detach_part_from_parent;
use crate::result::BaryError;
use crate::result::BaryResult;
use crate::sim::input_handlers;
use crate::sim::*;
use crate::sounds::*;
use crate::utils::*;
use bary_core::prelude::PI;
use bary_core::prelude::*;
use early_returns::ok_or_return;
use early_returns::some_or_return;
use log::*;
use rdev::Button;
use serde::{Deserialize, Serialize};
use std::collections::*;

#[derive(Serialize, Deserialize, Clone)]
pub struct World {
    pub ticks: u64,
    pub tick_rate: u32,

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
    pub stars: Components<Star>,

    // TODO might move this to assets.
    pub ship_names: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Star {
    pub pos: Vec3,
    pub alpha: f32,
}

pub fn spawn_stars(spawner: &mut EntitySpawner) -> Components<Star> {
    let n_stars = 4000;
    let mut stars = Components::default();
    for _ in 0..n_stars {
        let pos = randvec(0.0, 10000.0);
        let star = Star {
            pos: pos.extend(rand(0.3, 0.9)),
            alpha: rand(0.5, 1.0),
        };
        let id = spawner.spawn();
        stars.spawn(id, star);
    }
    stars
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
            spawner: EntitySpawner::default(),
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
            stars: Components::default(),
            ship_names: vec![
                "Gary".to_string(),
                "Sally".to_string(),
                "Juliet".to_string(),
                "Violet".to_string(),
                "Charlie".to_string(),
                "Orville".to_string(),
            ],
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

fn editor_offset_moves_with_wasd(input: &InputState, offset: &mut Vec2, zoom: f32) {
    let speed = 20.0 / zoom;

    if input.is_key_pressed(Key::ControlLeft) {
        return;
    }

    if input.is_key_pressed(Key::KeyS) {
        offset.y -= speed;
    }
    if input.is_key_pressed(Key::KeyW) {
        offset.y += speed;
    }
    if input.is_key_pressed(Key::KeyD) {
        offset.x += speed;
    }
    if input.is_key_pressed(Key::KeyA) {
        offset.x -= speed;
    }
}

fn camera_moves_with_wasd(
    input: &InputState,
    target: &mut Camera,
    follow: &mut Option<Ent>,
    lock_rotation: &mut bool,
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
        *lock_rotation = false;
    }
    if input.is_key_pressed(Key::KeyE) {
        target.isometry.rotation -= angular_speed;
        *lock_rotation = false;
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

fn editor_actual_offset_smooth_animation(target: Vec2, actual: &mut Vec2) {
    let rate_translation = 0.2;
    actual.x = low_pass(actual.x, target.x, rate_translation);
    actual.y = low_pass(actual.y, target.y, rate_translation);
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

pub fn destroy_part(world: &mut World, part_id: Ent) -> BaryResult<(PartInstance, Ent, Vec<Ent>)> {
    let (instance, grid_id) = destroy_part_without_integrity_check(world, part_id, true)?;
    let grids = split_grid_if_necessary(world, grid_id)?;
    Ok((instance, grid_id, grids))
}

pub fn destroy_part_batch(_world: &mut World, _parts: &[Ent]) -> BaryResult<()> {
    todo!()
}

pub fn explode_grid_at(loc: GridLocation, world: &mut World) {
    let p = loc.coord.inner();
    let r = 2;
    for x in p.x - r..=p.x + r {
        for y in p.y - r..=p.y + r {
            let mut loc = loc;
            loc.coord.0 = (x, y).into();
            _ = destroy_top_part_at(world, loc);
        }
    }
}

pub fn get_part_at(world: &World, loc: GridLocation, layer: PartLayer) -> BaryResult<Ent> {
    let grid = world.grids.try_get(loc.grid_id)?;
    let occ = grid
        .get_parts_at(loc.coord)
        .ok_or(BaryError::NoPartsAt(loc.coord))?;
    occ.at_layer(layer).ok_or(BaryError::NoPartsInLayer(layer))
}

pub fn get_top_part_at(world: &World, loc: GridLocation) -> BaryResult<Ent> {
    let grid = world.grids.try_get(loc.grid_id)?;
    grid.get_parts_at(loc.coord)
        .map(|occ| occ.top())
        .flatten()
        .ok_or(BaryError::NoPartsAt(loc.coord))
}

pub fn destroy_top_part_at(
    world: &mut World,
    loc: GridLocation,
) -> BaryResult<(PartInstance, Ent, Vec<Ent>)> {
    let top_part = get_top_part_at(world, loc)?;
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

fn propagate_grid_rigid_bodies(grids: &mut Components<VehicleGrid>) {
    for grid in grids.values_mut() {
        let body_frame_accel = grid.linear_acceleration();
        let omega = grid.angular_acceleration();
        let accel = rotate(body_frame_accel, grid.particle_location.rotation);
        grid.particle_location.translation += grid.velocity.translation * NOMINAL_DT;
        grid.velocity.translation += accel * NOMINAL_DT;
        grid.particle_location.rotation += grid.velocity.rotation * NOMINAL_DT;
        grid.velocity.rotation += omega * NOMINAL_DT;
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

fn update_actual_hover_part_info(client: &mut ClientSpecificInfo, grids: &Components<VehicleGrid>) {
    let mouse_screen_position = client.mouse_screen_position;
    let screen_dims = client.screen_dims;

    if let Some(free) = client.viewport.free_mut() {
        free.selection_info.hovered = None;
        let screen_pos = some_or_return!(mouse_screen_position);
        let world_pos = screen_to_world(&client.camera, screen_pos, screen_dims);
        let (grid_id, offset) = some_or_return!(find::closest_grid(grids, world_pos, None));
        let dist = offset.length();
        let grid = ok_or_return!(grids.try_get(grid_id));
        if 2.0 * grid.bounding_radius() < dist {
            return;
        }
        let origin = grid.origin();
        let coord = PartCoord::from_meters_floored(in_frame(origin, world_pos));
        free.selection_info.hovered = Some(GridLocation::new(grid_id, coord));
    } else if let Some(editor) = client.viewport.editor_mut() {
        editor.hovered = None;
        let grid = ok_or_return!(grids.try_get(editor.vehicle));
        let screen_pos = some_or_return!(mouse_screen_position);
        let world_pos = screen_to_world(&client.camera, screen_pos, screen_dims);
        // TODO(cleanup) completely unnecessary. shouldn't need to get the world coordinates
        // or the grid's coordinates to get this vector. just ask how far the camera is from
        // the grid in question!
        let local_pos = in_frame(grid.origin(), world_pos);
        let coord = PartCoord::from_meters_floored(local_pos);
        editor.hovered = Some(coord);
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

fn select_hovered_grid_loc_on_click(client: &mut ClientSpecificInfo, sounds: &mut SoundEffects) {
    let free = some_or_return!(client.viewport.free_mut());
    let old_grid = free.selection_info.first_selected_grid();

    let Some(hovered) = free.selection_info.hovered else {
        free.selection_info.selected.clear();
        return;
    };

    if client.input.is_key_pressed(Key::ShiftLeft) {
        free.selection_info.selected.push(hovered);
    } else {
        free.selection_info.selected = vec![hovered];
    }

    if free.selection_info.first_selected_grid().is_some() {
        sounds.push(SoundEffect::Open);
    } else if old_grid.is_some() {
        sounds.push(SoundEffect::Close);
    }
}

fn editor_on_release_left_click(client: &mut ClientSpecificInfo) {
    let e = some_or_return!(client.viewport.editor_mut());
    debug!("Editor left click release");
    e.select_start = None;
}

fn editor_on_left_click(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
) {
    let e = some_or_return!(client.viewport.editor_mut());

    debug!("Clicked on editor");

    e.select_start = None;

    let coord = some_or_return!(e.hovered);

    if let Some(proto_id) = e.prototype_id {
        let proto = ok_or_return!(world.prototypes.try_get(proto_id));

        let placement = GridPlacement::new(coord, e.part_rotation, proto.dims);

        let instance = PartInstance {
            name: proto.name.clone(),
            layer: proto.layer,
            placement,
        };

        let result = insert_part(e.vehicle, world, &instance, true);

        match result {
            Ok(ent) => {
                info!("Inserted part {ent}");
                sounds.push(SoundEffect::InsertPart);
            }
            Err(error) => {
                warn!("Failed to insert: {error:?}");
                sounds.push(SoundEffect::GenericFailure);
            }
        }
    } else {
        e.select_start = Some(coord);
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

    for (grid_id, grid) in grids.iter() {
        let is_controllable = !grid.computers.is_empty();
        if trackers.try_get(*grid_id).is_err() && is_controllable {
            let tracker = Tracker::default();
            trackers.spawn(*grid_id, tracker);
        }
    }

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
    world.ticks += 1;
    update_ring_particles(&mut world.particles);
    let dirty_set = update_thrusters(
        &mut world.thrusters,
        &world.grids,
        &world.parts,
        &world.computers,
    );

    world.grid_acceleration_updates += dirty_set.len() as u64;
    update_grid_acceleration_c(dirty_set, &mut world.grids, &world.thrusters, &world.parts);
    update_computers(&mut world.computers, &world.parts, &world.grids);
    propagate_grid_rigid_bodies(&mut world.grids);
    update_trackers(&mut world.tracking, &world.grids, world.ticks);
}

pub fn process_event(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    event: &rdev::Event,
) -> (SoundEffects, Vec<Action>) {
    let mut actions = Vec::new();
    let mut sounds = SoundEffects::new();

    if let rdev::EventType::KeyPress(k) = event.event_type {
        client.input.set_pressed(k);
    } else if let rdev::EventType::KeyRelease(k) = event.event_type {
        client.input.set_released(k);
    } else if let rdev::EventType::ButtonPress(mb) = event.event_type {
        client.input.set_pressed(mb);
    } else if let rdev::EventType::ButtonRelease(mb) = event.event_type {
        client.input.set_released(mb);
    }

    match event.event_type {
        rdev::EventType::KeyPress(key) => match key {
            Key::KeyS => input_handlers::save_on_ctrl_s(world, client),
            Key::KeyF => input_handlers::toggle_following_on_key_f(client, &mut sounds),
            Key::KeyT => input_handlers::toggle_tracking_for_selected_grid(world, client),
            Key::KeyR => {
                input_handlers::reset_camera_on_ctrl_r(client);
                input_handlers::lock_rotation_on_key_r(client);
                input_handlers::rotate_editor_part_on_key_r(client);
            }
            Key::Delete => input_handlers::destroy_selected_parts(world, client),
            Key::DownArrow => input_handlers::editor_layer_shift_on_page_key(client, false),
            Key::UpArrow => input_handlers::editor_layer_shift_on_page_key(client, true),
            Key::KeyE => input_handlers::editor_layer_shift_on_page_key(client, true),
            Key::KeyD => input_handlers::panic_on_ctrl_d(&mut client.input),
            Key::KeyP => input_handlers::spawn_random_ship_on_p(world),
            Key::KeyM => input_handlers::update_center_of_mass_on_m(world),
            Key::KeyQ => input_handlers::pipette_part_if_in_editor_on_q(world, client),
            Key::KeyG => input_handlers::enter_ship_editor(world, client, &mut sounds),
            Key::Escape => input_handlers::leave_ship_editor_on_escape(client, &mut sounds),
            Key::KeyC => {
                input_handlers::explode_at_mouseover(world, client);
                input_handlers::editor_copy_on_control_c(world, client);
            }
            _ => (),
        },
        rdev::EventType::KeyRelease(_key) => (),
        rdev::EventType::ButtonPress(button) => match button {
            Button::Left => {
                input_handlers::ping_on_alt_left_click(world, client, &mut actions, &mut sounds);
                select_hovered_grid_loc_on_click(client, &mut sounds);
                editor_on_left_click(world, client, &mut sounds);
            }
            Button::Right => {
                input_handlers::destroy_top_layer_part_at_mouseover(world, client, &mut sounds)
            }
            Button::Middle => {
                input_handlers::command_selected_ships_to_waypoint(world, client, &mut sounds)
            }
            Button::Unknown(_) => (),
        },
        rdev::EventType::ButtonRelease(button) => match button {
            Button::Left => editor_on_release_left_click(client),
            _ => (),
        },
        rdev::EventType::MouseMove { x: _, y: _ } => (),
        rdev::EventType::Wheel {
            delta_x: _,
            delta_y,
        } => {
            input_handlers::apply_scroll_wheel_to_camera_target(delta_y, &mut client.target_camera);
        }
    }

    (sounds, actions)
}

pub fn pre_simulation_update(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    _sounds: &mut SoundEffects,
) {
    client.ticks += 1;

    update_actual_hover_part_info(client, &world.grids);
}

fn test_button_boundaries_with_key_y(input: &InputState, sounds: &mut SoundEffects) {
    if input.just_pressed_debounced(Key::KeyY) {
        sounds.push(SoundEffect::Open);
    } else if input.just_released(Key::KeyY) {
        sounds.push(SoundEffect::Close);
    }
}

fn zoom_in_on_key_v(client: &mut ClientSpecificInfo) {
    if !client.input.just_pressed_debounced(Key::KeyV) {
        return;
    }

    let grid_id = some_or_return!(client.focused_grid_id());
    let free = some_or_return!(client.viewport.free_mut());
    if client.target_camera.zoom < ZOOM_NEAR_VEHICLE {
        client.target_camera.zoom = ZOOM_NEAR_VEHICLE;
    } else {
        client.target_camera.zoom = ZOOM_FAR_AWAY;
    }
    free.follow_vehicle = Some(grid_id);
}

pub fn post_simulation_update(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
) {
    client.chat.drop_old_messages();

    test_button_boundaries_with_key_y(&client.input, sounds);

    zoom_in_on_key_v(client);

    match &mut client.viewport {
        Viewport::Free(fly) => {
            set_target_camera_if_following(
                fly.follow_vehicle,
                fly.lock_rotation,
                &world.grids,
                &mut client.target_camera,
                &mut client.camera,
            );

            camera_moves_with_wasd(
                &client.input,
                &mut client.target_camera,
                &mut fly.follow_vehicle,
                &mut fly.lock_rotation,
                sounds,
            );

            camera_zooms_with_plus_minus(&client.input, &mut client.target_camera);
        }
        Viewport::Editor(editor) => {
            camera_zooms_with_plus_minus(&client.input, &mut client.target_camera);

            editor_offset_moves_with_wasd(
                &client.input,
                &mut editor.target_offset,
                client.camera.zoom,
            );

            editor_actual_offset_smooth_animation(editor.target_offset, &mut editor.actual_offset);

            set_cams_to_grid_pose(
                editor.vehicle,
                &world.grids,
                editor.actual_offset,
                &mut client.target_camera,
                &mut client.camera,
            );
        }
    }

    animate_camera_towards_target(&client.target_camera, &mut client.camera);
}

fn set_cams_to_grid_pose(
    grid_id: Ent,
    grids: &Components<VehicleGrid>,
    offset: Vec2,
    target: &mut Camera,
    actual: &mut Camera,
) {
    if let Ok(grid) = grids.try_get(grid_id) {
        target.isometry = grid.origin().offset(offset);
        target.zoom = target.zoom.clamp(EDITOR_MINIMUM_ZOOM, EDITOR_MAXIMUM_ZOOM);
        actual.isometry = target.isometry;
    }
}

fn update_ring_particles(particles: &mut Vec<PingParticle>) {
    for ring in particles.iter_mut() {
        ring.step()
    }
    particles.retain(|p| p.is_alive());
}
