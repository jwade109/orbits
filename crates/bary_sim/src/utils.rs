use crate::*;
use bary_core::prelude::*;
use bary_factory::*;
use bary_orbital::VehicleControl;
use bary_parts::*;
use early_returns::*;
use log::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub fn get_random_ship_name(names: &Vec<String>) -> String {
    if names.is_empty() {
        return String::new();
    }
    let idx = randint(0, names.len() as i32) as usize;
    names[idx].clone()
}

pub fn load_names_from_file(
    filename: impl AsRef<Path>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(filename)?
        .lines()
        .filter_map(|s| (!s.is_empty()).then(|| s.to_string()))
        .collect())
}

/// Spawns an empty vehicle grid.
pub fn spawn_empty_grid_c(
    spawner: &mut EntitySpawner,
    grids: &mut Components<VehicleGrid>,
    name: impl Into<String>,
) -> Ent {
    let name = name.into();
    debug!("Spawning empty grid with name {}", name);
    let grid = VehicleGrid::with_name(name, None);
    let id = spawner.spawn();
    grids.spawn(id, grid);
    id
}

/// Spawns an empty grid with the given name.
/// Exclusive version of [`super::spawn_empty_grid`].
pub fn spawn_empty_grid(world: &mut World, name: impl Into<String>) -> Ent {
    spawn_empty_grid_c(&mut world.spawner, &mut world.grids, name)
}

pub fn get_grid_physical_props(
    grid: &VehicleGrid,
    parts: &Components<Part>,
) -> BaryResult<(Mass, Vec2)> {
    let mut total_mass = Mass::ZERO;
    for part_id in &grid.parts {
        let part = parts.try_get(*part_id)?;
        total_mass += part.mass;
    }
    let mut com = Vec2::ZERO;
    for part_id in &grid.parts {
        let part = parts.try_get(*part_id)?;
        let center = part.region.center_isometry();
        let mass_portion = part.mass.to_kg_f64() / total_mass.to_kg_f64();
        com += center.translation * mass_portion as f32;
    }
    Ok((total_mass, com))
}

fn set_thruster_state_c(
    thruster_id: Ent,
    thrusters: &mut Components<Thruster>,
    new_state: bool,
) -> BaryResult<()> {
    let thruster = thrusters.try_get_mut(thruster_id)?;
    thruster.is_on = new_state;
    Ok(())
}

/// Sets the state of a given thruster.
/// Does not modify the corresponding grid's acceleration.
/// TODO(cleanup) this doesn't really need to be a function.
/// Exclusive version of [`set_thruster_state_c`].
pub fn set_thruster_state(thruster_id: Ent, world: &mut World, new_state: bool) -> BaryResult<()> {
    set_thruster_state_c(thruster_id, &mut world.thrusters, new_state)
}

pub fn get_top_part_at(world: &World, loc: GridLocation) -> BaryResult<Ent> {
    let grid = world.grids.try_get(loc.grid_id)?;
    grid.get_parts_at(loc.coord)
        .map(|occ| occ.top())
        .flatten()
        .ok_or(BaryError::NoPartsAt(loc.coord))
}

pub fn update_grid_physical_props(
    grid: &mut VehicleGrid,
    parts: &mut Components<Part>,
) -> BaryResult<()> {
    let offset = -grid.bounds.0;
    grid.occupancy.clear();
    grid.update_bounds();

    for part_id in grid.parts.clone() {
        let part = parts.try_get_mut(part_id)?;
        part.region.shift(offset.into());
        grid.mark_occupied(part.region, part.layer, part_id);
    }

    let old_com = grid.center_of_mass;

    let (mass, com) = get_grid_physical_props(grid, parts)?;

    let delta = com - old_com;

    info!("Delta COM: {} - {} = {}", com, old_com, delta);

    grid.particle_location.translation += rotate(delta, grid.particle_location.rotation);
    grid.parts_mass = mass;
    grid.center_of_mass = com;

    Ok(())
}

pub fn despawn_all_vehicles(world: &mut World) -> usize {
    let ret = world.grids.len();
    world.grids.clear();
    world.parts.clear();
    world.lights.clear();
    world.computers.clear();
    world.thrusters.clear();
    ret
}

pub fn despawn_grid_c(
    grid_id: Ent,
    grids: &mut Components<VehicleGrid>,
    parts: &mut Components<Part>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    lights: &mut Components<Light>,
) -> BaryResult<()> {
    let grid = grids.despawn(grid_id)?;
    for id in grid.parts {
        parts.despawn(id)?;
    }
    for id in grid.thrusters {
        thrusters.despawn(id)?;
    }
    for id in grid.computers {
        computers.despawn(id)?;
    }
    for id in grid.lights {
        lights.despawn(id)?;
    }
    Ok(())
}

/// Produces the entity ID corresponding to a grid's primary CPU,
/// which by convention is just the first element in the computer index.
pub fn get_primary_cpu_id(grid_id: Ent, grids: &Components<VehicleGrid>) -> BaryResult<Ent> {
    let grid = grids.try_get(grid_id)?;
    Ok(*grid.computers.first().ok_or(BaryError::NoPrimaryComputer)?)
}

pub fn get_thruster_levels(
    grid_id: Ent,
    grids: &Components<VehicleGrid>,
    thrusters: &Components<Thruster>,
) -> BaryResult<Vec<(Ent, bool)>> {
    let grid = grids.try_get(grid_id)?;
    let mut results = Vec::new();
    for thruster_id in &grid.thrusters {
        let thruster = thrusters.try_get(*thruster_id)?;
        results.push((*thruster_id, thruster.is_on));
    }
    Ok(results)
}

/// Updates the `body_frame_forces` field of a particular
/// vehicle grid specified by the entity ID.
pub fn update_single_grid_acceleration(
    grid_id: Ent,
    grids: &mut Components<VehicleGrid>,
    thrusters: &Components<Thruster>,
    parts: &Components<Part>,
) -> BaryResult<()> {
    let grid = grids.try_get_mut(grid_id)?;
    grid.body_frame_forces = Isometry2d::ZERO;

    if grid.is_anchored {
        return Ok(());
    }

    for thruster_id in &grid.thrusters {
        let thruster = thrusters.try_get(*thruster_id)?;

        if !thruster.is_on {
            continue;
        }

        let part = parts.try_get(*thruster_id)?;

        let center_of_thrust = part.region.center_isometry().translation;
        let rotation = part.region.rot();
        let wrench = body_frame_wrench(
            thruster.thrust,
            center_of_thrust,
            rotation,
            grid.center_of_mass,
        );
        grid.body_frame_forces.translation += wrench.translation;
        grid.body_frame_forces.rotation += wrench.rotation;
    }

    Ok(())
}

pub fn sys_update_grid_acceleration_c(
    dirty_set: BTreeSet<Ent>,
    grids: &mut Components<VehicleGrid>,
    thrusters: &Components<Thruster>,
    parts: &Components<Part>,
) {
    for grid_id in dirty_set {
        if let Err(e) = update_single_grid_acceleration(grid_id, grids, thrusters, parts) {
            error!("Failed to update grid accel: {e:?}");
        }
    }
}

pub fn update_grid_physical_props_by_id(
    grid_id: Ent,
    grids: &mut Components<VehicleGrid>,
    parts: &mut Components<Part>,
) -> BaryResult<()> {
    let grid = grids.try_get_mut(grid_id)?;
    update_grid_physical_props(grid, parts)
}

pub fn get_grid_physical_props_by_id(
    grid_id: Ent,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
) -> BaryResult<(Mass, Vec2)> {
    let grid = grids.try_get(grid_id)?;
    get_grid_physical_props(grid, parts)
}

pub fn set_grid_pose(world: &mut World, grid_id: Ent, iso: Isometry2d) -> BaryResult<()> {
    info!("Setting isometry of grid {} to {:?}", grid_id, iso);
    let grid = world.grids.try_get_mut(grid_id)?;
    grid.particle_location = iso;
    Ok(())
}

pub fn set_grid_vel(world: &mut World, grid_id: Ent, vel: Isometry2d) -> BaryResult<()> {
    info!("Setting velocity of grid {} to {:?}", grid_id, vel);
    let grid = world.grids.try_get_mut(grid_id)?;
    grid.velocity = vel;
    Ok(())
}

/// Inserts a part into an existing grid.
pub fn insert_part(
    grid_id: Ent,
    world: &mut World,
    instance: &PartInstance,
    update_props: bool,
) -> BaryResult<Ent> {
    let grid = world.grids.try_get_mut(grid_id)?;

    if !grid.can_insert_part(instance.region, instance.layer) {
        warn!("Can't insert part!");
        return Err(BaryError::GridSpaceOccupied);
    }

    let proto_id =
        get_proto_by_name(&world.prototypes, &instance.name).ok_or(BaryError::BadPartName)?;
    let proto = world.prototypes.try_get(proto_id)?;

    let part = Part {
        region: instance.region,
        layer: instance.layer(),
        mass: proto.mass,
        prototype: proto_id,
        grid_id,
        classification: proto.classification(),
    };

    let part_id = world.spawner.spawn();

    grid.parts.insert(part_id);
    world.parts.spawn(part_id, part);

    grid.mark_occupied(instance.region, instance.layer(), part_id);

    if let Some(inv) = &proto.inventory_data {
        let slots = inv
            .slots
            .iter()
            .map(|data| {
                InvSlot::new(
                    Volume::liters_f32(data.volume_liters),
                    data.filter.clone(),
                    data.is_fluid.unwrap_or(false),
                    (data.min.into(), data.max.into()),
                )
            })
            .collect();

        let inventory = Inventory::from_slots(slots);
        world.inventories.spawn(part_id, inventory);
    }
    if let Some(data) = &proto.machine_data {
        let machine = data.clone().into_machine();
        world.machines.spawn(part_id, machine);
    }
    if let Some(data) = &proto.thruster_data {
        let thruster = Thruster {
            is_on: false,
            is_rcs: data.is_rcs,
            // TODO(gross)
            thrust: data.thrust as f32,
            last_controlled_by: None,
        };
        world.thrusters.spawn(part_id, thruster);
        grid.thrusters.insert(part_id);
    }
    if let Some(_data) = &proto.computer_data {
        let cpu = Computer::new(proto_id);
        world.computers.spawn(part_id, cpu);
        grid.computers.insert(part_id);
    }
    if let Some(data) = &proto.thruster_data {
        if data.is_rcs {
            let light_idx = world.lights.len();
            let light = Light::new(light_idx as u32);
            world.lights.spawn(part_id, light);
            grid.lights.insert(part_id);
        }
    }
    if let Some(debug) = &proto.debug_portal_data {
        let portal = DebugPortal::from_proto(*debug);
        world.debug_portals.spawn(part_id, portal);
    }
    if let Some(ex) = &proto.excavator_data {
        let excavator = Excavator::from_proto(ex);
        world.excavators.spawn(part_id, excavator);
    }

    if update_props {
        update_grid_physical_props_by_id(grid_id, &mut world.grids, &mut world.parts)?;
    }

    Ok(part_id)
}

/// Produces whatever prototype has the given name, if any.
pub fn get_proto_by_name(prototypes: &Components<PartPrototype>, name: &str) -> Option<Ent> {
    prototypes
        .iter()
        .find(|(_, proto)| proto.part_name() == name)
        .map(|e| *e.0)
}

/// Removes a part from its parent grid, updating any relevant quantities
/// about that grid. This does not perform an integrity check, and might
/// leave the parent grid in a state where it should be split up into
/// several grids.
///
/// TODO(bug) It also doesn't update the grid's acceleration, so if a thruster
/// is removed while it's firing that acceleration will remain until the grid's
/// acceleration is recalculated.
///
/// Returns the grid which was modified, if any.
///
/// TODO(testing) very, VERY important to test!
pub fn destroy_part_without_integrity_check(
    world: &mut World,
    part_id: Ent,
    update_props: bool,
) -> BaryResult<(PartInstance, Ent)> {
    let part = world.parts.try_get(part_id)?;
    let grid_id = part.grid_id;
    let proto = world.prototypes.try_get(part.prototype)?;
    let grid = world.grids.try_get_mut(grid_id)?;

    let instance = PartInstance::new(&proto.name, proto.layer, part.region);

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
    grid.parts_mass -= part.mass;

    world.parts.despawn(part_id)?;

    if grid.parts.is_empty() {
        world.grids.despawn(grid_id)?;
    }

    if update_props {
        update_grid_physical_props_by_id(grid_id, &mut world.grids, &mut world.parts)?;
    }

    Ok((instance, grid_id))
}

pub fn duplicate_part_to_new_grid(world: &mut World, part_id: Ent) -> BaryResult<Ent> {
    let part = world.parts.try_get(part_id)?;
    let grid = world.grids.try_get(part.grid_id)?;
    let new_name = format!("{}-debris", grid.name);
    let new_part_pose = grid.origin() * part.region.origin_isometry();
    let new_grid_vel = grid.velocity + Isometry2d::new(randvec(1.0, 3.0), rand(-0.1, 0.1));
    let proto = world.prototypes.try_get(part.prototype)?;
    let instance = PartInstance {
        name: proto.name.clone(),
        layer: proto.layer,
        region: GridRegion::new((0, 0), Rotation::East, proto.dims),
    };
    let new_grid_id = spawn_empty_grid(world, new_name);
    set_grid_pose(world, new_grid_id, new_part_pose)?;
    set_grid_vel(world, new_grid_id, new_grid_vel)?;
    let new_part_id = insert_part(new_grid_id, world, &instance, true)?;
    Ok(new_part_id)
}

pub fn detach_part_from_parent(world: &mut World, part_id: Ent) -> BaryResult<Ent> {
    let part = world.parts.try_get(part_id)?;
    let grid_id = part.grid_id;
    let new_part_id = duplicate_part_to_new_grid(world, part_id)?;
    destroy_part_without_integrity_check(world, part_id, true)?;
    split_grid_if_necessary(world, grid_id)?;
    Ok(new_part_id)
}

pub fn split_grid_if_necessary(world: &mut World, grid_id: Ent) -> BaryResult<Vec<Ent>> {
    let grid = world.grids.try_get(grid_id)?;
    let islands = grid.calculate_islands();
    if islands.is_empty() {
        // TODO debatable
        return Ok(vec![]);
    }
    if islands.len() == 1 {
        return Ok(vec![]);
    }

    let mut key_parts = BTreeMap::new();
    for island in &islands {
        let Some(key_part) = island.first() else {
            continue;
        };
        let part = world.parts.try_get(*key_part)?;
        let pose = grid.origin() * part.region.origin_isometry();
        info!("Position of {}: {:?}", key_part, pose);
        key_parts.insert(*key_part, pose);
    }

    let mut ids = Vec::new();
    let rebuilt = rebuild_index_from_islands(grid, &islands, &world.parts)?;

    for (i, r) in rebuilt.into_iter().enumerate() {
        if i == 0 {
            world.grids.update(grid_id, r);
            ids.push(grid_id);
        } else {
            let id = world.spawner.spawn();
            for part_id in &r.parts {
                if let Ok(part) = world.parts.try_get_mut(*part_id) {
                    part.grid_id = id;
                }
            }
            world.grids.spawn(id, r);
            ids.push(id);
        }
    }

    for id in &ids {
        update_grid_physical_props_by_id(*id, &mut world.grids, &mut world.parts)?;
    }

    for island in &islands {
        let Some(key_part) = island.first() else {
            continue;
        };
        let Some(old_pose) = key_parts.get(key_part) else {
            continue;
        };
        let part = world.parts.try_get(*key_part)?;
        let grid = world.grids.try_get_mut(part.grid_id)?;
        let pose = grid.origin() * part.region.origin_isometry();
        let delta = pose.translation - old_pose.translation;
        info!("Position of {}: {:?}, was {:?}", key_part, pose, old_pose);
        grid.particle_location.translation -= delta;
    }

    Ok(ids)
}

pub fn rebuild_index_from_island(
    src: &VehicleGrid,
    island: &BTreeSet<Ent>,
    parts: &Components<Part>,
    name: String,
) -> BaryResult<VehicleGrid> {
    let mut dst = VehicleGrid::with_name(name, None);
    dst.particle_location = src.particle_location;
    dst.velocity = src.velocity;
    for part_id in island.iter().map(|i| *i) {
        let part = parts.try_get(part_id)?;
        dst.parts_mass += part.mass;
        dst.parts.insert(part_id);
        dst.mark_occupied(part.region, part.layer, part_id);
        if src.thrusters.contains(&part_id) {
            dst.thrusters.insert(part_id);
        }
        if src.computers.contains(&part_id) {
            dst.computers.insert(part_id);
        }
        if src.lights.contains(&part_id) {
            dst.lights.insert(part_id);
        }
    }

    Ok(dst)
}

pub fn rebuild_index_from_islands(
    src: &VehicleGrid,
    islands: &Vec<BTreeSet<Ent>>,
    parts: &Components<Part>,
) -> BaryResult<Vec<VehicleGrid>> {
    islands
        .iter()
        .map(|island| rebuild_index_from_island(src, island, parts, src.name.clone()))
        .collect()
}

pub fn set_player_piloting_grid(world: &mut World, player_id: Ent, grid_id: Ent) -> BaryResult<()> {
    let player = world.players.try_get_mut(player_id)?;
    player.state = PlayerState::PilotingGrid(grid_id, VehicleControl::NULLOPT);
    Ok(())
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

pub fn toggle_tracking(world: &mut World, grid_id: Ent) -> BaryResult<bool> {
    let tracking = if world.tracking.has_entity(grid_id) {
        world.tracking.despawn(grid_id)?;
        info!("Removed tracking for grid {}", grid_id);
        false
    } else {
        world.tracking.spawn(grid_id, Tracker::default());
        info!("Enabled tracking for grid {}", grid_id);
        true
    };
    Ok(tracking)
}

pub fn set_grid_anchored(world: &mut World, grid_id: Ent, anchored: bool) -> BaryResult<()> {
    let grid = world.grids.try_get_mut(grid_id)?;
    grid.is_anchored = anchored;
    grid.velocity = Isometry2d::ZERO;
    Ok(())
}

pub fn grid_set_waypoint_to_asteroid_center(
    world: &mut World,
    grid_id: Ent,
    ast_id: Ent,
) -> BaryResult<()> {
    let ast = world.asteroids.try_get(ast_id)?;
    set_primary_computer_waypoint(grid_id, ast.iso, world)?;
    set_primary_computer_state_c(grid_id, true, &world.grids, &mut world.computers)?;
    Ok(())
}

/// Sets the waypoint field of the primary computer,
/// if the provided grid has one. If it does, the ID of the primary
/// computer will be returned.
/// TODO(testing) test this.
pub fn set_primary_computer_waypoint(
    grid_id: Ent,
    waypoint: impl Into<Isometry2d>,
    world: &mut World,
) -> BaryResult<Ent> {
    super::set_primary_computer_waypoint_c(grid_id, waypoint, &world.grids, &mut world.computers)
}

/// Sets the waypoint field of the primary computer,
/// if the provided grid has one. If it does, the ID of the primary
/// computer will be returned.
/// TODO(testing) test this.
pub fn set_primary_computer_waypoint_c(
    grid_id: Ent,
    waypoint: impl Into<Isometry2d>,
    grids: &Components<VehicleGrid>,
    computers: &mut Components<Computer>,
) -> BaryResult<Ent> {
    let primary_cpu_id = get_primary_cpu_id(grid_id, grids)?;
    let computer = computers.try_get_mut(primary_cpu_id)?;
    let command = TimedInstruction::perp(Instruction::HoldPosition(waypoint.into()));
    computer.command_queue = vec![command];
    Ok(primary_cpu_id)
}

/// Turns the primary computer of the given grid on or off,
/// returning the entity ID of the computer if it was found.
/// TODO(testing) test this.
pub fn set_primary_computer_state_c(
    grid_id: Ent,
    new_state: bool,
    grids: &Components<VehicleGrid>,
    computers: &mut Components<Computer>,
) -> BaryResult<Ent> {
    let primary_cpu_id = get_primary_cpu_id(grid_id, grids)?;
    let computer = computers.try_get_mut(primary_cpu_id)?;
    computer.on = new_state;
    Ok(primary_cpu_id)
}

/// Turns the primary computer of the given grid on or off,
/// returning the entity ID of the computer if it was found.
pub fn set_primary_computer_state(
    grid_id: Ent,
    new_state: bool,
    world: &mut World,
) -> BaryResult<Ent> {
    set_primary_computer_state_c(grid_id, new_state, &world.grids, &mut world.computers)
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

pub fn destroy_part(world: &mut World, part_id: Ent) -> BaryResult<(PartInstance, Ent, Vec<Ent>)> {
    let (instance, grid_id) = destroy_part_without_integrity_check(world, part_id, true)?;
    let grids = split_grid_if_necessary(world, grid_id)?;
    Ok((instance, grid_id, grids))
}

pub fn get_part_at(world: &World, loc: GridLocation, layer: PartLayer) -> BaryResult<Ent> {
    let grid = world.grids.try_get(loc.grid_id)?;
    let occ = grid
        .get_parts_at(loc.coord)
        .ok_or(BaryError::NoPartsAt(loc.coord))?;
    occ.at_layer(layer).ok_or(BaryError::NoPartsInLayer(layer))
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

pub fn get_grid_origin(grids: &Components<VehicleGrid>, grid_id: Ent) -> Option<Isometry2d> {
    let grid = grids.try_get(grid_id).ok()?;
    Some(grid.origin())
}

pub fn grid_pose(grids: &Components<VehicleGrid>, grid_id: Ent) -> Option<Isometry2d> {
    let grid = grids.try_get(grid_id).ok()?;
    Some(grid.particle_location)
}

pub fn grid_vel(grids: &Components<VehicleGrid>, grid_id: Ent) -> Option<Isometry2d> {
    let grid = grids.try_get(grid_id).ok()?;
    Some(grid.velocity)
}

pub fn part_pose(
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    part_id: Ent,
) -> BaryResult<Isometry2d> {
    let part = parts.try_get(part_id)?;
    let grid = grids.try_get(part.grid_id)?;
    Ok(grid.origin() * part.region.center_isometry())
}

pub fn gridloc_pose(grids: &Components<VehicleGrid>, loc: GridLocation) -> BaryResult<Isometry2d> {
    let grid = grids.try_get(loc.grid_id)?;
    Ok(grid.origin().offset(loc.coord.to_meters()))
}

pub fn despawn_grid(world: &mut World, grid_id: Ent) -> BaryResult<()> {
    despawn_grid_c(
        grid_id,
        &mut world.grids,
        &mut world.parts,
        &mut world.thrusters,
        &mut world.computers,
        &mut world.lights,
    )
}

pub fn sum_part_masses(
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    prototypes: &Components<PartPrototype>,
    grid_id: Ent,
) -> BaryResult<(Mass, Vec2)> {
    let grid = grids.try_get(grid_id)?;
    let mut sum = Mass::ZERO;
    let mut com = DVec2::ZERO;
    for part_id in &grid.parts {
        let part = parts.try_get(*part_id)?;
        let proto = prototypes.try_get(part.prototype)?;
        sum += proto.mass;
        let center = part.region.center_isometry().translation;
        com += center.as_dvec2();
    }
    if !sum.is_zero() {
        com /= sum.to_kg_f64();
    }
    Ok((sum, com.as_vec2()))
}

pub fn sum_part_masses_w(world: &World, grid_id: Ent) -> BaryResult<(Mass, Vec2)> {
    sum_part_masses(&world.grids, &world.parts, &world.prototypes, grid_id)
}

pub fn get_blueprint_by_id<'a>(
    blueprints: &'a Components<NamedBlueprint>,
    id: &BlueprintId,
) -> Option<&'a Blueprint> {
    let result = blueprints.values().find(|bp| &bp.id == id);

    if result.is_none() {
        error!("Failed to get blueprint with ID {:?}", id);
    }

    result.map(|e| &e.blueprint)
}

pub fn spawn_player(world: &mut World, username: String, iso: Isometry2d) -> BaryResult<Ent> {
    let already_exists = world.players.values().any(|player| player.name == username);

    if already_exists {
        return Err(BaryError::PlayerAlreadyExists(username));
    }

    let player = Player {
        name: username,
        cursor_world_position: None,
        state: PlayerState::Flying(iso),
    };
    let id = world.spawner.spawn();
    world.players.spawn(id, player);
    Ok(id)
}

pub fn set_player_position(world: &mut World, player_id: Ent, iso: Isometry2d) -> BaryResult<()> {
    let player = world.players.try_get_mut(player_id)?;
    player.set_position(iso);
    Ok(())
}

pub fn set_player_cursor_position(
    world: &mut World,
    player_id: Ent,
    pos: Option<Vec2>,
) -> BaryResult<()> {
    let player = world.players.try_get_mut(player_id)?;
    player.cursor_world_position = pos;
    Ok(())
}

pub fn player_exit_grid(world: &mut World, player_id: Ent) -> BaryResult<()> {
    let player = world.players.try_get_mut(player_id)?;
    let grid_id = player
        .driving_grid()
        .ok_or(BaryError::PlayerNotDriving(player_id))?;
    let grid = world.grids.try_get(grid_id)?;
    player.set_position(grid.particle_location);
    Ok(())
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

pub fn insert_pipe(
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

pub fn update_world(world: &mut World) -> DebugTimers {
    world.ticks += 1;

    let mut timers = DebugTimers::default();
    timers.ticks += 1;

    {
        let _timer = timers.scope("grid_motion");

        sys_update_ring_particles(&mut world.particles, world.ticks);
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

    {
        let _timer = timers.scope("terrain_mining");

        sys_mine_tiles(world);
    }

    timers
}

pub fn get_slot_mut_c<'a>(
    loc: GridLocation,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    inventories: &'a mut Components<Inventory>,
) -> BaryResult<&'a mut InvSlot> {
    let (part_id, slot_id) = inventory_at_c(loc, grids, parts, inventories)?;
    let inv = inventories.try_get_mut(part_id)?;
    inv.get_slot_mut(slot_id)
        .ok_or(BaryError::NoInvSlot(slot_id))
}

/// Fills inventories which have debug sources attached.
fn sys_fill_inventories_attached_to_debug_sources(world: &mut World) {
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

/// Performs inventory transfers according to the pipes that exist
/// in the world.
fn sys_update_pipes(inventories: &mut Components<Inventory>, pipes: &mut Components<Pipe>) {
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

/// Steps running machines forward by one tick, and modifies their
/// corresponding inventory if necessary.
fn sys_update_machines(world: &mut World) {
    for (part_id, machine) in world.machines.iter_mut() {
        step_process(machine, *part_id, &mut world.inventories);
    }
}

/// Iterates over all active excavators and removes tiles accordingly
fn sys_mine_tiles(world: &mut World) {
    let mut to_remove: BTreeMap<Ent, BTreeSet<GlobalTileIndex>> = BTreeMap::new();
    for (id, ex) in world.excavators.iter() {
        let Ok(Some((ast_id, tiles))) = get_excavator_tiles(*id, ex, world) else {
            continue;
        };
        to_remove
            .entry(ast_id)
            .and_modify(|e| {
                for t in &tiles {
                    e.insert(*t);
                }
            })
            .or_insert(BTreeSet::from_iter(tiles));
    }

    for (ast_id, tiles) in to_remove {
        for t in tiles {
            _ = remove_terrain_tile(world, ast_id, t);
        }
    }
}

pub fn get_slot_c<'a>(
    loc: GridLocation,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    inventories: &'a Components<Inventory>,
) -> BaryResult<&'a InvSlot> {
    let (part_id, slot_id) = inventory_at_c(loc, grids, parts, inventories)?;
    let inv = inventories.try_get(part_id)?;
    inv.get_slot(slot_id).ok_or(BaryError::NoInvSlot(slot_id))
}

pub fn inventory_at_c(
    loc: GridLocation,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    inventories: &Components<Inventory>,
) -> BaryResult<(Ent, usize)> {
    let grid = grids.try_get(loc.grid_id)?;
    let occ = grid
        .get_parts_at(loc.coord)
        .ok_or(BaryError::NoPartsAt(loc.coord))?;
    let part_id = occ
        .at_layer(PartLayer::Internal)
        .ok_or(BaryError::NoPartsInLayer(PartLayer::Internal))?;
    let inv = inventories.try_get(part_id)?;
    let part = parts.try_get(part_id)?;
    let local = part.region.to_local(loc.coord);
    let idx = inv
        .get_slot_at(local)
        .ok_or(BaryError::NoInvAt(loc.coord))?;
    Ok((part_id, idx))
}

/// Returns the ID of the first grid in the components list with
/// the given name.
///
/// Buyer beware: grid names are not unique! This
/// only promises to return any grid with the given name, if one exists.
pub fn get_grid_by_name(grids: &Components<VehicleGrid>, name: &str) -> Option<Ent> {
    grids
        .iter()
        .find_map(|(id, grid)| (grid.name == name).then(|| *id))
}

/// Spawns a new grid according to a named blueprint.
pub fn spawn_grid_with_bp_id(
    world: &mut World,
    bp_id: &BlueprintId,
    grid_name: &str,
) -> BaryResult<Ent> {
    let bp = get_blueprint_by_id(&world.blueprints, bp_id)
        .ok_or(BaryError::BadBlueprint)?
        .clone();
    spawn_grid_from_blueprint(world, grid_name, Some(bp_id), &bp)
}

/// Spawns a grid according to the given blueprint.
pub fn spawn_grid_from_blueprint(
    world: &mut World,
    grid_name: impl Into<String>,
    bp_id: Option<&BlueprintId>,
    bp: &Blueprint,
) -> BaryResult<Ent> {
    let grid_name = grid_name.into();
    info!(
        "Spawning grid with name \"{}\" from blueprint {:?}",
        grid_name, bp_id
    );
    let grid = VehicleGrid::with_name(grid_name, bp_id.cloned());
    let grid_id = world.spawner.spawn();
    world.grids.spawn(grid_id, grid.clone());
    for (_id, instance) in bp.parts() {
        insert_part(grid_id, world, instance, false)?;
    }

    for (_id, pipe) in bp.pipes() {
        if let Err(e) = insert_pipe_at_c(
            grid_id,
            pipe.start,
            pipe.end,
            &mut world.spawner,
            &mut world.grids,
            &mut world.parts,
            &mut world.inventories,
            &mut world.pipes,
        ) {
            error!("Failed to insert pipe: {:?}", e);
        }
    }

    let grid = world.grids.try_get_mut(grid_id)?;

    update_grid_physical_props(grid, &mut world.parts)?;

    Ok(grid_id)
}

pub fn spawn_grid_with_random_name(
    world: &mut World,
    bp_id: impl Into<BlueprintId>,
) -> BaryResult<Ent> {
    let name = get_random_ship_name(&world.ship_names);
    let bp_id = bp_id.into();
    spawn_grid_with_bp_id(world, &bp_id, &name)
}

pub fn can_insert_part_c(
    grid_id: Ent,
    pl: GridRegion,
    layer: PartLayer,
    grids: &Components<VehicleGrid>,
) -> BaryResult<bool> {
    let grid = grids.try_get(grid_id)?;
    Ok(grid.can_insert_part(pl, layer))
}

pub fn get_closest_grid(
    grids: &Components<VehicleGrid>,
    test_pos: Vec2,
    dist_limit: impl Into<Option<f32>>,
) -> Option<(Ent, Vec2)> {
    let mut best: Option<(Ent, Vec2, f32)> = None;
    let dist_limit = dist_limit.into().unwrap_or(std::f32::INFINITY);
    for (e, grid) in grids.iter() {
        let in_frame = express_in_frame(grid.centroid_isometry(), test_pos);
        let dist = in_frame.length_squared();
        if dist > dist_limit {
            continue;
        }
        if let Some(best) = &mut best {
            if dist < best.2 {
                best.0 = *e;
                best.1 = in_frame;
                best.2 = dist;
            }
        } else {
            best = Some((*e, in_frame, dist));
        }
    }
    best.map(|x| (x.0, x.1))
}

pub fn get_blueprint_c(
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    pipes: &Components<Pipe>,
    prototypes: &Components<PartPrototype>,
    grid_id: Ent,
) -> BaryResult<Blueprint> {
    let grid = grids.try_get(grid_id)?;
    let mut bp = Blueprint::new();
    for part_id in &grid.parts {
        let part = parts.try_get(*part_id)?;
        let proto = prototypes.try_get(part.prototype)?;
        bp.add_part(proto.name.to_string(), part.region, part.layer);
    }
    for pipe_id in &grid.pipes {
        let pipe = pipes.try_get(*pipe_id)?;
        let src_part = parts.try_get(pipe.src.part_id)?;
        let dst_part = parts.try_get(pipe.dst.part_id)?;
        let start = src_part.region.to_global(pipe.src.offset);
        let end = dst_part.region.to_global(pipe.dst.offset);

        assert_eq!(src_part.region.to_local(start), pipe.src.offset);
        assert_eq!(dst_part.region.to_local(end), pipe.dst.offset);

        let geo = PipeGeometry {
            start,
            end,
            x_first: false,
        };
        bp.add_pipe(geo);
    }
    Ok(bp)
}

pub fn get_sum_linear_forces(
    grid_id: Ent,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    thrusters: &Components<Thruster>,
) -> BaryResult<Vec2> {
    let grid = grids.try_get(grid_id)?;
    let mut sum = Vec2::ZERO;
    for part_id in &grid.thrusters {
        let thruster = thrusters.try_get(*part_id)?;
        let part = parts.try_get(*part_id)?;
        let thrust = rotate(Vec2::X, part.region.rot().to_angle() as f32) * thruster.thrust;
        sum += thrust;
    }
    Ok(sum)
}
