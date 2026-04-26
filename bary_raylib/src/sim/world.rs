use crate::camera::Camera;
use crate::client::*;
use crate::components::*;
use crate::constants::*;
use crate::imgui::*;
use crate::multiplayer::Action;
use crate::ops::destroy_part_without_integrity_check;
use crate::ops::detach_part_from_parent;
use crate::sim::find::get_slot_mut_c;
use crate::sim::input_handlers;
use crate::sim::*;
use crate::sounds::*;
use crate::utils::*;
use bary_core::prelude::PI;
use bary_core::prelude::*;
use early_returns::*;
use log::*;
use rdev::Button;
use serde::{Deserialize, Serialize};
use std::collections::*;
use std::time::Duration;
use std::time::Instant;

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
    pub inventories: Components<Inventory>,
    pub machines: Components<Machine>,
    pub stars: Components<Star>,
    pub pipes: Components<Pipe>,
    pub debug_portals: Components<DebugPortal>,

    // TODO might move this to assets.
    pub ship_names: Vec<String>,
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
            inventories: Components::default(),
            machines: Components::default(),
            stars: Components::default(),
            pipes: Components::default(),
            debug_portals: Components::default(),
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
    let speed = 40.0 / zoom;

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

pub fn destroy_part_at_layer(
    world: &mut World,
    loc: GridLocation,
    layer: PartLayer,
) -> BaryResult<(PartInstance, Ent, Vec<Ent>)> {
    let part_id = get_part_at(world, loc, layer)?;
    destroy_part(world, part_id)
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

fn sys_propagate_grid_rigid_bodies(grids: &mut Components<VehicleGrid>) {
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

fn sys_update_thrusters(
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

            let tac = match part.region.rot() {
                Rotation::East => ctrl.plus_x,
                Rotation::North => ctrl.neg_y,
                Rotation::West => ctrl.neg_x,
                Rotation::South => ctrl.plus_y,
            };

            // TODO(optimization) reduce lookups by storing isometry on the thruster?
            let isometry = part.region.center_isometry();
            let center_of_thrust = isometry.translation;
            let rotation = part.region.rot();
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

pub fn get_part_at_layer(
    grid: &VehicleGrid,
    coord: PartCoord,
    layer: PartLayer,
) -> BaryResult<Ent> {
    grid.get_parts_at(coord)
        .ok_or(BaryError::NoPartsAt(coord))?
        .at_layer(PartLayer::Internal)
        .ok_or(BaryError::NoPartsInLayer(layer))
}

pub fn calculate_pipe_joint_c(
    loc: GridLocation,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    inventories: &Components<Inventory>,
) -> BaryResult<PipeJoint> {
    let grid = grids.try_get(loc.grid_id)?;
    let part_id = get_part_at_layer(grid, loc.coord, PartLayer::Internal)?;
    let part_a = parts.try_get(part_id)?;
    let local = part_a.region.to_local(loc.coord);
    let src_inv = inventories
        .try_get(part_id)
        .map_err(|_| BaryError::PartHasNoInv(part_id))?;
    let slot = src_inv
        .get_slot_at(local)
        .ok_or(BaryError::NoInvAt(loc.coord))?;

    Ok(PipeJoint {
        part_id,
        offset: local,
        slot,
    })
}

pub fn insert_pipe_at_c(
    grid_id: Ent,
    src: PartCoord,
    dst: PartCoord,
    spawner: &mut EntitySpawner,
    grids: &mut Components<VehicleGrid>,
    parts: &Components<Part>,
    inventories: &Components<Inventory>,
    pipes: &mut Components<Pipe>,
) -> BaryResult<(Pipe, Ent)> {
    if src == dst {
        return Err(BaryError::ZeroPipeExtent);
    }

    let src_loc = GridLocation::new(grid_id, src);
    let dst_loc = GridLocation::new(grid_id, dst);

    let src_joint = calculate_pipe_joint_c(src_loc, grids, parts, inventories)?;
    let dst_joint = calculate_pipe_joint_c(dst_loc, grids, parts, inventories)?;

    let grid = grids.try_get_mut(grid_id)?;

    if src_joint.part_id == dst_joint.part_id && src_joint.slot == dst_joint.slot {
        return Err(BaryError::SameInvSlot(src_joint.part_id, src_joint.slot));
    }

    let pipe = Pipe {
        src: src_joint,
        dst: dst_joint,
        status: MachineStatus::Off,
    };

    let id = spawner.spawn();
    pipes.spawn(id, pipe);

    grid.pipes.insert(id);

    Ok((pipe, id))
}

pub fn insert_pipe_at(
    grid_id: Ent,
    src: PartCoord,
    dst: PartCoord,
    world: &mut World,
) -> BaryResult<(Pipe, Ent)> {
    insert_pipe_at_c(
        grid_id,
        src,
        dst,
        &mut world.spawner,
        &mut world.grids,
        &world.parts,
        &world.inventories,
        &mut world.pipes,
    )
}

fn editor_on_release_left_click(client: &mut ClientSpecificInfo, world: &mut World) {
    let e = some_or_return!(client.viewport.editor_mut());
    debug!("Editor left click release");

    let src = e.select_start;
    let dst = e.hovered;

    if let (Some(src), Some(dst)) = (src, dst) {
        if e.layer == Some(PartLayer::Plumbing) {
            match insert_pipe_at(e.vehicle, src, dst, world) {
                Ok((pipe, _id)) => {
                    let s = format!("{:?}", pipe);
                    client.chat.log(s);
                }
                Err(e) => {
                    let s = format!("Failed to insert pipe: {:?}", e);
                    client.chat.log(s);
                }
            }
        }
    }

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

        let region = GridRegion::new(coord, e.part_rotation, proto.dims);

        let instance = PartInstance {
            name: proto.name.clone(),
            layer: proto.layer,
            region,
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

pub fn sys_update_trackers(
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

pub fn sys_fill_inventories_attached_to_debug_sources(world: &mut World) {
    for (part_id, portal) in world.debug_portals.iter() {
        let part = ok_or_continue!(world.parts.try_get(*part_id));
        let loc = GridLocation::new(part.grid_id, part.region.origin());
        let slot = ok_or_continue!(get_slot_mut_c(
            loc,
            &world.grids,
            &world.parts,
            &mut world.inventories
        ));

        match portal.state {
            PortalState::Source(item) => {
                if let Some(item) = item {
                    slot.fill_with(item);
                }
            }
            PortalState::Sink => {
                slot.empty();
            }
        }
    }
}

pub fn set_inventory_slot(
    inventories: &mut Components<Inventory>,
    slot: InvSlot,
    inv_id: Ent,
    slot_id: usize,
) -> BaryResult<()> {
    let inv = inventories.try_get_mut(inv_id)?;
    let old_slot = inv
        .get_slot_mut(slot_id)
        .ok_or(BaryError::NoInvSlot(slot_id))?;
    *old_slot = slot;
    Ok(())
}

pub fn sys_update_pipes(inventories: &mut Components<Inventory>, pipes: &mut Components<Pipe>) {
    for pipe in pipes.values_mut() {
        let inv_a = ok_or_continue!(inventories.try_get(pipe.src.part_id));
        let inv_b = ok_or_continue!(inventories.try_get(pipe.dst.part_id));

        let mut src = some_or_continue!(inv_a.get_slot(pipe.src.slot)).clone();
        let mut dst = some_or_continue!(inv_b.get_slot(pipe.dst.slot)).clone();

        if src.is_empty() {
            pipe.status = MachineStatus::Starved;
            continue;
        }

        let mass = {
            let mul = randint(140, 160);
            let m = src.mass() / mul as u64;
            if m.is_zero() { Mass::grams(1) } else { m }
        };

        pipe.status = atomic_transfer(&mut src, &mut dst, mass);

        _ = set_inventory_slot(inventories, src, pipe.src.part_id, pipe.src.slot);
        _ = set_inventory_slot(inventories, dst, pipe.dst.part_id, pipe.dst.slot);
    }
}

pub fn step_process(machine: &mut Machine, id: Ent, inv: &mut Components<Inventory>) {
    if machine.recipe().is_none() {
        machine.status = MachineStatus::NoRecipe;
        return;
    }

    if !machine.enabled {
        machine.status = MachineStatus::Off;
        return;
    }

    if machine.steps == 0 {
        if let Ok(inv) = inv.try_get_mut(id) {
            if machine.take_inputs_if_possible(inv) {
                machine.steps += 1;
                machine.status = MachineStatus::Running;
                return;
            } else {
                machine.status = MachineStatus::Starved;
                return;
            }
        } else {
            machine.status = MachineStatus::Starved;
            return;
        }
    }

    if machine.steps > 0 && machine.steps < machine.required_steps {
        machine.status = MachineStatus::Running;
        machine.steps += 1;
    } else if machine.steps >= machine.required_steps {
        if let Ok(inv) = inv.try_get_mut(id) {
            if machine.store_outputs_if_possible(inv) {
                machine.steps = 0;
                machine.products_finished += 1;
                machine.status = MachineStatus::Running;
            } else {
                machine.status = MachineStatus::NoRoom;
            }
        } else {
            machine.status = MachineStatus::NoRoom;
        }
    } else {
        machine.status = MachineStatus::Off;
    }
}

pub fn sys_update_machines(world: &mut World) {
    for (part_id, machine) in world.machines.iter_mut() {
        step_process(machine, *part_id, &mut world.inventories);
    }
}

#[derive(Debug, Default)]
pub struct DebugTimers {
    pub ticks: u64,
    pub timers: BTreeMap<String, Duration>,
}

impl DebugTimers {
    pub fn total(&self) -> Duration {
        self.timers.iter().map(|e| e.1).sum()
    }

    pub fn scope<'a>(&'a mut self, name: &str) -> ScopeTimer<'a> {
        ScopeTimer {
            timers: self,
            name: name.to_string(),
            start: Instant::now(),
        }
    }
}

impl std::ops::AddAssign for DebugTimers {
    fn add_assign(&mut self, rhs: Self) {
        self.ticks += rhs.ticks;
        for (k, v) in rhs.timers {
            self.timers.entry(k).and_modify(|e| *e += v).or_insert(v);
        }
    }
}

pub struct ScopeTimer<'a> {
    timers: &'a mut DebugTimers,
    name: String,
    start: Instant,
}

impl<'a> Drop for ScopeTimer<'a> {
    fn drop(&mut self) {
        let dur = Instant::now() - self.start;
        self.timers
            .timers
            .entry(self.name.clone())
            .and_modify(|e| *e += dur)
            .or_insert(dur);
    }
}

pub fn update_world(world: &mut World) -> DebugTimers {
    world.ticks += 1;

    let mut timers = DebugTimers::default();
    timers.ticks += 1;

    {
        let _timer = timers.scope("grid_motion");

        sys_update_ring_particles(&mut world.particles);
        let dirty_set = sys_update_thrusters(
            &mut world.thrusters,
            &world.grids,
            &world.parts,
            &world.computers,
        );

        world.grid_acceleration_updates += dirty_set.len() as u64;
        sys_update_grid_acceleration_c(dirty_set, &mut world.grids, &world.thrusters, &world.parts);
        sys_update_computers(&mut world.computers, &world.parts, &world.grids);
        sys_propagate_grid_rigid_bodies(&mut world.grids);
    }

    {
        let _timer = timers.scope("update_trackers");

        sys_update_trackers(&mut world.tracking, &world.grids, world.ticks);
    }

    {
        let _timer = timers.scope("update_pipes");

        sys_update_pipes(&mut world.inventories, &mut world.pipes);
    }

    {
        let _timer = timers.scope("fill_inventories");

        sys_fill_inventories_attached_to_debug_sources(world);
    }

    {
        let _timer = timers.scope("update_machines");

        sys_update_machines(world);
    }

    timers
}

pub fn consume_rdev_event_into_input_state(input: &mut InputState, event: &rdev::Event) {
    if let rdev::EventType::KeyPress(k) = event.event_type {
        input.set_pressed(k);
    } else if let rdev::EventType::KeyRelease(k) = event.event_type {
        input.set_released(k);
    } else if let rdev::EventType::ButtonPress(mb) = event.event_type {
        input.set_pressed(mb);
    } else if let rdev::EventType::ButtonRelease(mb) = event.event_type {
        input.set_released(mb);
    }
}

pub fn process_event(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    event: &rdev::Event,
    sounds: &mut SoundEffects,
    actions: &mut Vec<Action>,
    on_gui: bool,
) {
    match event.event_type {
        rdev::EventType::KeyPress(key) => match key {
            Key::KeyS => input_handlers::save_on_ctrl_s(world, client),
            Key::KeyF => input_handlers::toggle_following_on_key_f(client, sounds),
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
            Key::KeyG => input_handlers::enter_ship_editor(world, client, sounds),
            Key::Escape => input_handlers::leave_ship_editor_on_escape(client, sounds),
            Key::KeyC => {
                input_handlers::explode_at_mouseover(world, client);
                input_handlers::editor_copy_on_control_c(world, client);
            }
            _ => (),
        },
        rdev::EventType::KeyRelease(_key) => (),
        rdev::EventType::ButtonPress(button) => {
            if !on_gui {
                match button {
                    Button::Left => {
                        input_handlers::ping_on_alt_left_click(world, client, actions, sounds);
                        select_hovered_grid_loc_on_click(client, sounds);
                        editor_on_left_click(world, client, sounds);
                    }
                    Button::Right => {
                        input_handlers::destroy_top_layer_part_at_mouseover(world, client, sounds)
                    }
                    Button::Middle => (),
                    Button::Unknown(_) => (),
                }
            }
        }
        rdev::EventType::ButtonRelease(button) => match button {
            Button::Left => editor_on_release_left_click(client, world),
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
}

pub fn pre_simulation_update(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
) {
    client.ticks += 1;

    update_actual_hover_part_info(client, &world.grids);

    if client.input.just_pressed_debounced(Key::Alt) {
        client.alt_mode ^= true;
    }

    if client.input.just_pressed_debounced(Button::Right) {
        if let Some(mouse_pos) = client.mouse_screen_position {
            if let Some(free) = client.viewport.free_mut() {
                let world_pos = screen_to_world(&client.camera, mouse_pos, client.screen_dims);
                free.waypoint_widget = Some(world_pos);
            }
        }
    }

    if client.input.just_released(Button::Right) {
        if let Some(free) = client.viewport.free() {
            if let Some(p) = free.waypoint_widget {
                if let Some(mouse_pos) = client.mouse_screen_position {
                    let q = screen_to_world(&client.camera, mouse_pos, client.screen_dims);
                    input_handlers::command_selected_ships_to_waypoint(world, client, sounds, p, q);
                }
            }
        }

        if let Some(free) = client.viewport.free_mut() {
            free.waypoint_widget = None;
        }
    }
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

fn sys_update_ring_particles(particles: &mut Vec<PingParticle>) {
    for ring in particles.iter_mut() {
        ring.step()
    }
    particles.retain(|p| p.is_alive());
}
