use crate::*;
use bary_core::prelude::*;
use bary_factory::*;
use bary_parts::*;
use log::*;
use std::{collections::{BTreeMap, BTreeSet}, path::Path};

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
