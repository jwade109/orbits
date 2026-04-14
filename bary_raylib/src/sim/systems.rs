use crate::assets::get_random_ship_name;
use crate::components::*;
use crate::result::*;
use crate::sim::*;
use bary_core::prelude::*;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use log::*;
use std::collections::BTreeSet;
use std::time::Duration;

pub const TICKS_PER_SECOND: u64 = 50;

pub fn apparent_elapsed_time(world: &World) -> Duration {
    Duration::from_millis(1000 / TICKS_PER_SECOND * world.ticks)
}

pub fn apparent_datetime(world: &World) -> NaiveDateTime {
    let dur = apparent_elapsed_time(world);
    let epoch = NaiveDate::from_ymd_opt(2310, 7, 8)
        .unwrap()
        .and_hms_opt(3, 0, 0)
        .unwrap();
    epoch + dur
}

pub fn spawn_grid_from_blueprint_c(
    counter: &mut EntitySpawner,
    prototypes: &Components<PartPrototype>,
    grids: &mut Components<VehicleGrid>,
    parts: &mut Components<Part>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    lights: &mut Components<Light>,
    inventories: &mut Components<Inventory>,
    machines: &mut Components<Machine>,
    debug_portals: &mut Components<DebugPortal>,
    name: impl Into<String>,
    bp: &Blueprint,
) -> BaryResult<Ent> {
    let s = name.into();
    info!("Spawning grid with name \"{}\" from blueprint", s);
    let grid = VehicleGrid::with_name(s);
    let grid_id = counter.spawn();
    grids.spawn(grid_id, grid.clone());
    for (_id, proto) in bp.parts() {
        insert_part_c(
            grid_id,
            counter,
            grids,
            prototypes,
            parts,
            thrusters,
            computers,
            lights,
            inventories,
            machines,
            debug_portals,
            proto,
            false,
        )?;
    }

    let grid = grids.try_get_mut(grid_id)?;

    update_grid_physical_props(grid, parts)?;

    Ok(grid_id)
}

pub fn body_frame_wrench(
    thrust: f32,
    center_of_thrust: Vec2,
    rotation: Rotation,
    com: Vec2,
) -> Isometry2d {
    let u = rotation.to_dir();
    let lever_arm = center_of_thrust - com;
    let thrust = thrust * u.as_vec2();
    let torque = cross2d(lever_arm, thrust);
    Isometry2d::new(thrust, torque as f32)
}

pub fn update_grid_acceleration_c(
    dirty_set: BTreeSet<Ent>,
    grids: &mut Components<VehicleGrid>,
    thrusters: &Components<Thruster>,
    parts: &Components<Part>,
) {
    for grid_id in dirty_set {
        let Ok(grid) = grids.try_get_mut(grid_id) else {
            continue;
        };
        grid.body_frame_forces = Isometry2d::ZERO;
        for thruster_id in &grid.thrusters {
            let Ok(thruster) = thrusters.try_get(*thruster_id) else {
                continue;
            };

            if !thruster.is_on {
                continue;
            }

            let Ok(part) = parts.try_get(*thruster_id) else {
                continue;
            };

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
    }
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
    let primary_cpu_id = find::primary_computer_id(grid_id, grids)?;
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
    let primary_cpu_id = find::primary_computer_id(grid_id, grids)?;
    let computer = computers.try_get_mut(primary_cpu_id)?;
    computer.on = new_state;
    Ok(primary_cpu_id)
}

/// Spawns an empty grid with the given name.
/// Exclusive version of [`super::spawn_empty_grid`].
pub fn spawn_empty_grid(world: &mut World, name: impl Into<String>) -> Ent {
    spawn_empty_grid_c(&mut world.spawner, &mut world.grids, name)
}

pub fn toggle_tracking(world: &mut World, grid_id: Ent) -> BaryResult<bool> {
    let tracking = if world.tracking.contains_key(&grid_id) {
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

/// Turns the primary computer of the given grid on or off,
/// returning the entity ID of the computer if it was found.
pub fn set_primary_computer_state(
    grid_id: Ent,
    new_state: bool,
    world: &mut World,
) -> BaryResult<Ent> {
    super::set_primary_computer_state_c(grid_id, new_state, &world.grids, &mut world.computers)
}

pub fn set_all_thrusters(grid_id: Ent, new_state: bool, world: &mut World) -> BaryResult<()> {
    let grid = world.grids.try_get(grid_id)?;
    for thruster_id in &grid.thrusters {
        let thruster = world.thrusters.try_get_mut(*thruster_id)?;
        thruster.is_on = new_state;
    }
    update_grid_acceleration([grid_id].into(), world);
    Ok(())
}

pub fn update_grid_acceleration(dirty_set: BTreeSet<Ent>, world: &mut World) {
    update_grid_acceleration_c(dirty_set, &mut world.grids, &world.thrusters, &world.parts);
}

/// Spawns a grid according to the given blueprint.
/// Exclusive version of [`super::spawn_grid_from_blueprint`].
pub fn spawn_grid_from_blueprint(
    world: &mut World,
    name: impl Into<String>,
    bp: &Blueprint,
) -> BaryResult<Ent> {
    spawn_grid_from_blueprint_c(
        &mut world.spawner,
        &mut world.prototypes,
        &mut world.grids,
        &mut world.parts,
        &mut world.thrusters,
        &mut world.computers,
        &mut world.lights,
        &mut world.inventories,
        &mut world.machines,
        &mut world.debug_portals,
        name,
        bp,
    )
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

pub fn spawn_grid_with_random_name(world: &mut World, bp_name: &str) -> BaryResult<Ent> {
    let name = get_random_ship_name(&world.ship_names);
    spawn_grid_by_name(world, bp_name, &name)
}

/// Spawns a new grid according to a named blueprint.
pub fn spawn_grid_by_name(world: &mut World, bp_name: &str, name: &str) -> BaryResult<Ent> {
    let bp = find::blueprint_by_name(&world.blueprints, bp_name)
        .ok_or(BaryError::BadBlueprint)?
        .clone();
    spawn_grid_from_blueprint(world, name, &bp)
}

/// Sets the state of a given thruster.
/// Does not modify the corresponding grid's acceleration.
/// TODO(cleanup) this doesn't really need to be a function.
/// Exclusive version of [`set_thruster_state_c`].
pub fn set_thruster_state(thruster_id: Ent, world: &mut World, new_state: bool) -> BaryResult<()> {
    set_thruster_state_c(thruster_id, &mut world.thrusters, new_state)
}

pub fn ping(world: &mut World, pos: Vec2) {
    let part = PingParticle::new(pos);
    world.particles.push(part);
}

pub fn get_blueprint(world: &World, grid_id: Ent) -> BaryResult<Blueprint> {
    get_blueprint_c(&world.grids, &world.parts, &world.prototypes, grid_id)
}

/// Spawns an empty vehicle grid.
pub fn spawn_empty_grid_c(
    spawner: &mut EntitySpawner,
    grids: &mut Components<VehicleGrid>,
    name: impl Into<String>,
) -> Ent {
    let name = name.into();
    debug!("Spawning empty grid with name {}", name);
    let grid = VehicleGrid::with_name(name);
    let id = spawner.spawn();
    grids.spawn(id, grid);
    id
}

/// Inserts a part into an existing grid.
/// Exclusive version of [`insert_part_c`].
pub fn insert_part(
    grid_id: Ent,
    world: &mut World,
    instance: &PartInstance,
    update_props: bool,
) -> BaryResult<Ent> {
    insert_part_c(
        grid_id,
        &mut world.spawner,
        &mut world.grids,
        &mut world.prototypes,
        &mut world.parts,
        &mut world.thrusters,
        &mut world.computers,
        &mut world.lights,
        &mut world.inventories,
        &mut world.machines,
        &mut world.debug_portals,
        instance,
        update_props,
    )
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

pub fn insert_part_c(
    grid_id: Ent,
    counter: &mut EntitySpawner,
    grids: &mut Components<VehicleGrid>,
    prototypes: &Components<PartPrototype>,
    parts: &mut Components<Part>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    lights: &mut Components<Light>,
    inventories: &mut Components<Inventory>,
    machines: &mut Components<Machine>,
    debug_portals: &mut Components<DebugPortal>,
    instance: &PartInstance,
    update_props: bool,
) -> BaryResult<Ent> {
    let grid = grids.try_get_mut(grid_id)?;

    if !grid.can_insert_part(instance.region, instance.layer) {
        warn!("Can't insert part!");
        return Err(BaryError::GridSpaceOccupied);
    }

    let proto_id = find::proto_by_name(prototypes, &instance.name).ok_or(BaryError::BadPartName)?;
    let proto = prototypes.try_get(proto_id)?;

    let part = Part {
        region: instance.region,
        layer: instance.layer(),
        mass: proto.mass,
        prototype: proto_id,
        grid_id,
        classification: proto.classification(),
    };

    let part_id = counter.spawn();

    grid.parts.insert(part_id);
    parts.spawn(part_id, part);

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
        inventories.spawn(part_id, inventory);
    }
    if let Some(data) = &proto.machine_data {
        let machine = Machine::from_data(data.clone());
        machines.spawn(part_id, machine);
    }
    if let Some(data) = &proto.thruster_data {
        let thruster = Thruster {
            is_on: false,
            is_rcs: data.is_rcs,
            // TODO(gross)
            thrust: data.thrust as f32,
            last_controlled_by: None,
        };
        thrusters.spawn(part_id, thruster);
        grid.thrusters.insert(part_id);
    }
    if let Some(_data) = &proto.computer_data {
        let cpu = Computer::new(proto_id);
        computers.spawn(part_id, cpu);
        grid.computers.insert(part_id);
    }
    if let Some(data) = &proto.thruster_data {
        if data.is_rcs {
            let light_idx = lights.len();
            let light = Light::new(light_idx as u32);
            lights.spawn(part_id, light);
            grid.lights.insert(part_id);
        }
    }
    if let Some(debug) = &proto.debug_portal_data {
        let portal = DebugPortal::from_proto(*debug);
        debug_portals.spawn(part_id, portal);
    }

    if update_props {
        update_grid_physical_props_by_id(grid_id, grids, parts)?;
    }

    Ok(part_id)
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

pub mod find {
    use log::error;

    use crate::client::GridLocation;

    use super::*;

    /// Produces the entity ID corresponding to a grid's primary CPU,
    /// which by convention is just the first element in the computer index.
    pub fn primary_computer_id(grid_id: Ent, grids: &Components<VehicleGrid>) -> BaryResult<Ent> {
        let grid = grids.try_get(grid_id)?;
        Ok(*grid.computers.first().ok_or(BaryError::NoPrimaryComputer)?)
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

    pub fn blueprint_by_name<'a>(
        blueprints: &'a Components<NamedBlueprint>,
        name: &str,
    ) -> Option<&'a Blueprint> {
        let result = blueprints
            .values()
            .find(|(n, _bp)| n == name)
            .map(|(_, bp)| bp);

        if result.is_none() {
            error!("Failed to get blueprint with name {}", name);
        }

        result
    }

    /// Produces whatever prototype has the given name, if any.
    pub fn proto_by_name(prototypes: &Components<PartPrototype>, name: &str) -> Option<Ent> {
        prototypes
            .iter()
            .find(|(_, proto)| proto.part_name() == name)
            .map(|e| *e.0)
    }

    pub fn grid_origin(grids: &Components<VehicleGrid>, grid_id: Ent) -> Option<Isometry2d> {
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

    pub fn gridloc_pose(
        grids: &Components<VehicleGrid>,
        loc: GridLocation,
    ) -> BaryResult<Isometry2d> {
        let grid = grids.try_get(loc.grid_id)?;
        Ok(grid.origin().offset(loc.coord.to_meters()))
    }

    /// Returns the ID of the first grid in the components list with
    /// the given name.
    ///
    /// Buyer beware: grid names are not unique! This
    /// only promises to return any grid with the given name, if one exists.
    pub fn grid_by_name(grids: &Components<VehicleGrid>, name: &str) -> Option<Ent> {
        grids
            .iter()
            .find_map(|(id, grid)| (grid.name == name).then(|| *id))
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

    pub fn closest_grid(
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
}

pub fn get_blueprint_c(
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
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

pub fn update_grid_physical_props_by_id(
    grid_id: Ent,
    grids: &mut Components<VehicleGrid>,
    parts: &mut Components<Part>,
) -> BaryResult<()> {
    let grid = grids.try_get_mut(grid_id)?;
    update_grid_physical_props(grid, parts)
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

pub fn get_grid_physical_props_by_id(
    grid_id: Ent,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
) -> BaryResult<(Mass, Vec2)> {
    let grid = grids.try_get(grid_id)?;
    get_grid_physical_props(grid, parts)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        query::grid_origin, sim::find::grid_pose, tests::assert_world_is_consistent,
        world_builder::WorldBuilder,
    };

    #[test]
    fn part_prototypes() {
        let world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let mut iter = world.prototypes.iter();

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(0));
        assert_eq!(proto.part_name(), "angled-frame");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(1));
        assert_eq!(proto.part_name(), "antenna");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(2));
        assert_eq!(proto.part_name(), "battery");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(3));
        assert_eq!(proto.part_name(), "cargo");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(4));
        assert_eq!(proto.part_name(), "chemical-plant");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(5));
        assert_eq!(proto.part_name(), "container");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(6));
        assert_eq!(proto.part_name(), "cpu");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(7));
        assert_eq!(proto.part_name(), "debug-item-source");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(8));
        assert_eq!(proto.part_name(), "debug-sink");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(9));
        assert_eq!(proto.part_name(), "debug-source");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(10));
        assert_eq!(proto.part_name(), "docking-port");

        assert_world_is_consistent(&world);
    }

    #[test]
    fn vehicle_spawning_and_despawning() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let name = "pollux";

        // get the blueprint for the pollux
        let bp = find::blueprint_by_name(&world.blueprints, name)
            .expect("Expected a blueprint")
            .clone();

        // spawn that vehicle using its blueprint
        let grid_id = spawn_grid_from_blueprint(&mut world, name.to_string(), &bp)
            .expect("Expected the grid ID");

        let expected_grid_id = Ent(37);

        // this entity should be the same every time
        assert_eq!(grid_id, expected_grid_id);

        // the mass should already be computed
        let grid = world.grids.get(expected_grid_id).unwrap();
        assert_eq!(grid.parts_mass, Mass::grams(35134000));

        assert_eq!(world.grids.len(), 1);
        assert_eq!(world.parts.len(), 98);
        assert_eq!(world.thrusters.len(), 18);
        assert_eq!(world.computers.len(), 1);
        assert_eq!(world.lights.len(), 12);

        // get the computer entity
        let (id, cpu) = world.computers.iter().next().unwrap();

        // these entities should be the same every time
        assert_eq!(*id, Ent(61));
        assert_eq!(cpu.prototype, Ent(6));

        // get the prototype definition for the computer
        let proto = world.prototypes.get(cpu.prototype).unwrap();

        // it should be the "cpu" part
        assert_eq!(proto.part_name(), "cpu");

        // despawning should work, of course
        let result = despawn_grid(&mut world, grid_id);
        assert_eq!(result, Ok(()));

        // now the world should be empty
        assert_eq!(world.grids.len(), 0);
        assert_eq!(world.parts.len(), 0);
        assert_eq!(world.thrusters.len(), 0);
        assert_eq!(world.computers.len(), 0);
        assert_eq!(world.lights.len(), 0);

        // doing this again should return an error
        let result = despawn_grid(&mut world, grid_id);

        assert_eq!(result, Err(BaryError::EntityNotFound(grid_id)));

        assert_world_is_consistent(&world);
    }

    #[test]
    fn nearest_grid() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        assert!(find::closest_grid(&world.grids, Vec2::new(100.0, 200.0), None).is_none());

        let id = spawn_grid_with_random_name(&mut world, "remora").unwrap();
        assert_eq!(id, Ent(37));

        let grid = world.grids.try_get_mut(id).unwrap();
        grid.particle_location.translation = Vec2::new(40.0, 156.0);
        grid.particle_location.rotation = 30.0f32.to_radians();

        let centroid = grid.centroid_isometry();

        for _ in 0..100 {
            update_world(&mut world);
            let test_pos = centroid.offset(Vec2::new(100.0, 200.0)).translation;
            let e = find::closest_grid(&world.grids, test_pos, None);
            assert_eq!(e, Some((Ent(37), Vec2::new(99.99999, 199.99998))));
        }

        assert_world_is_consistent(&world);
    }

    #[test]
    fn insert_parts() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .build();

        let initial_id = world.spawner.next();

        let part_name = "motor";

        let proto_id = find::proto_by_name(&world.prototypes, part_name).unwrap();

        let proto = world.prototypes.try_get(proto_id).unwrap();
        let dims = proto.dims;

        assert_eq!(proto_id, initial_id - 16);

        let grid_id = spawn_grid_with_random_name(&mut world, "pollux").unwrap();

        assert_eq!(world.parts.len(), 98);
        assert_eq!(world.thrusters.len(), 18);

        assert_eq!(grid_id, initial_id);

        let instance = PartInstance::new(
            part_name,
            PartLayer::Internal,
            GridRegion::new((2, 20), Rotation::East, dims),
        );

        let id = insert_part(grid_id, &mut world, &instance, true).unwrap();

        assert_world_is_consistent(&world);

        assert_eq!(id, initial_id + 99);

        let part = world.parts.get(id).unwrap();

        assert_eq!(part.grid_id, grid_id);
        assert_eq!(part.prototype, proto_id);
        assert_eq!(
            part.region,
            // TODO allow insertion at a given region
            GridRegion::new((2, 20), Rotation::East, (6, 3))
        );

        assert_eq!(world.parts.len(), 99);
        assert_eq!(world.thrusters.len(), 19);
    }

    #[test]
    fn parts_mass() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let id = spawn_grid_with_random_name(&mut world, "pollux").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(35134000));

        let id = spawn_grid_with_random_name(&mut world, "bellerophon").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(178051000));

        let id = spawn_grid_with_random_name(&mut world, "remora").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(12339000));

        let id = spawn_grid_with_random_name(&mut world, "spacestation").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(145638000));

        assert_world_is_consistent(&world);
    }

    #[test]
    fn calculate_blueprints() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let mut expected = find::blueprint_by_name(&world.blueprints, "pollux")
            .unwrap()
            .clone();

        expected.normalize_coordinates();

        let id = spawn_grid_with_random_name(&mut world, "pollux").unwrap();

        let actual = get_blueprint_c(&world.grids, &world.parts, &world.prototypes, id).unwrap();

        assert_eq!(actual.part_count(), expected.part_count());

        for (a, b) in actual.parts().zip(expected.parts()) {
            assert_eq!(a.0, b.0);
            assert_eq!(a.1, b.1);
        }

        // TODO this test needs to be revived once pipes are added
        // to the new ECS
        // assert_eq!(actual.pipe_count(), expected.pipe_count());

        // failing because pipes aren't implemented.
        // assert_eq!(actual, expected);

        assert_world_is_consistent(&world);
    }

    #[test]
    fn bad_part_insertion() {
        let mut world = WorldBuilder::new().test_assets().build();
        let id = spawn_empty_grid(&mut world, "whatever");

        let instance = PartInstance::new(
            "dingus",
            PartLayer::Internal,
            GridRegion::new((0, 0), Rotation::East, (3, 3)),
        );

        let result = insert_part(id, &mut world, &instance, true);

        assert_eq!(result, Err(BaryError::BadPartName));

        let instance = PartInstance::new(
            "cargo",
            PartLayer::Internal,
            GridRegion::new((0, 0), Rotation::East, (3, 3)),
        );

        let result = insert_part(Ent(103), &mut world, &instance, true);

        assert_eq!(result, Err(BaryError::EntityNotFound(Ent(103))));

        assert_world_is_consistent(&world);
    }

    #[test]
    fn setting_thruster_state() {
        let mut world = WorldBuilder::new().test_assets().build();

        let grid_id = spawn_empty_grid(&mut world, "whatever");

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts.len(), 0);
        assert_eq!(grid.parts_mass, Mass::ZERO);

        assert_eq!(grid_id, Ent(33));

        let instance_a = PartInstance::new(
            "motor",
            PartLayer::Internal,
            GridRegion::new((0, 0), Rotation::East, (6, 3)),
        );

        let instance_b = PartInstance::new(
            "small-motor",
            PartLayer::Internal,
            GridRegion::new((3, 3), Rotation::North, (4, 2)),
        );

        let a_id = insert_part(grid_id, &mut world, &instance_a, true).unwrap();
        let b_id = insert_part(grid_id, &mut world, &instance_b, true).unwrap();

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts.len(), 2);
        assert_eq!(grid.parts, [a_id, b_id].into());
        assert_eq!(grid.thrusters, [a_id, b_id].into());
        assert_eq!(grid.parts_mass, Mass::grams(870000));

        assert_eq!(a_id, Ent(34));
        assert_eq!(b_id, Ent(35));

        let r1 = set_thruster_state(a_id, &mut world, true);
        let r2 = set_thruster_state(b_id, &mut world, true);

        update_grid_acceleration([grid_id].into(), &mut world);

        assert_eq!(r1, Ok(()));
        assert_eq!(r2, Ok(()));

        let sum =
            get_sum_linear_forces(grid_id, &world.grids, &world.parts, &world.thrusters).unwrap();

        assert_eq!(sum.x, 400000.0);
        assert_eq!(sum.y, 320000.0);

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(
            grid.body_frame_forces.translation,
            Vec2::new(400000.0, 320000.0)
        );

        let r1 = set_thruster_state(a_id, &mut world, false);
        let r2 = set_thruster_state(b_id, &mut world, true);

        update_grid_acceleration([grid_id].into(), &mut world);

        assert_eq!(r1, Ok(()));
        assert_eq!(r2, Ok(()));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.body_frame_forces.translation, Vec2::new(0.0, 320000.0));

        let r1 = set_thruster_state(a_id, &mut world, false);
        let r2 = set_thruster_state(b_id, &mut world, false);

        update_grid_acceleration([grid_id].into(), &mut world);

        assert_eq!(r1, Ok(()));
        assert_eq!(r2, Ok(()));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(
            grid.body_frame_forces.translation,
            IVec2::new(0, 0).as_vec2()
        );
    }

    #[test]
    fn parts_center_of_mass() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .build();

        let id = spawn_grid_with_random_name(&mut world, "pollux").unwrap();
        let (_mass, com) = get_grid_physical_props_by_id(id, &world.grids, &world.parts).unwrap();

        assert_eq!(com, Vec2::new(5.5010653, 2.272271));

        let cargo_id = find::proto_by_name(&world.prototypes, "cargo").unwrap();
        let cargo_proto = world.prototypes.try_get(cargo_id).unwrap();

        assert_eq!(cargo_proto.dims, (6, 6).into());
        assert_eq!(cargo_proto.dims_meters(), (1.5, 1.5).into());

        let instance = PartInstance::from_prototype(cargo_proto, (0, 0).into(), Rotation::East);

        let grid_id = spawn_empty_grid(&mut world, "whatever");
        _ = insert_part(grid_id, &mut world, &instance, true);

        let (_mass, com) =
            get_grid_physical_props_by_id(grid_id, &world.grids, &world.parts).unwrap();

        assert_eq!(com, Vec2::splat(0.75));
    }

    #[test]
    fn adding_part_to_empty_grid_retains_origin_pose() {
        // this caught me by surprise, but this is actually perfectly consistent behavior;
        // adding a part to an empty grid will retain the _origin_ of that grid,
        // NOT the center of mass, in the inertial frame.

        let mut world = WorldBuilder::new().test_assets().build();

        let grid_id = spawn_empty_grid(&mut world, "empty");
        let pose = grid_pose(&world.grids, grid_id).unwrap();
        let origin = grid_origin(&world.grids, grid_id).unwrap();
        assert_eq!(pose, Isometry2d::ZERO);
        assert_eq!(origin, Isometry2d::ZERO);

        // insert a frame
        let instance = PartInstance {
            name: "frame".to_string(),
            layer: PartLayer::Structural,
            region: GridRegion::new((1, 1), Rotation::East, (2, 2)),
        };

        let result = insert_part(grid_id, &mut world, &instance, true);
        assert!(result.is_ok());

        let part_dims = instance.region.grid_aligned_dims().to_meters();

        assert_eq!(part_dims, Vec2::splat(0.5));

        let pose = grid_pose(&world.grids, grid_id).unwrap();
        let origin = grid_origin(&world.grids, grid_id).unwrap();
        assert_eq!(pose, (part_dims / 2.0, 0.0).into());
        assert_eq!(origin, Isometry2d::ZERO);

        assert_world_is_consistent(&world);
    }

    #[test]
    fn pure_linear_acceleration() {
        let mut world = WorldBuilder::new().test_assets().build();

        // modifying the prototype for motor so it has easy quantities
        let proto_id = find::proto_by_name(&world.prototypes, "small-motor").unwrap();
        let proto = world.prototypes.try_get_mut(proto_id).unwrap();

        proto.mass = Mass::kilograms(1000);
        if let Some(t) = &mut proto.thruster_data {
            // 3500 newtons
            t.thrust = 3500.0;
        }

        let dims = proto.dims;

        let grid_id = spawn_empty_grid(&mut world, "testbed");

        let instance = PartInstance {
            name: "small-motor".to_string(),
            layer: PartLayer::Internal,
            region: GridRegion::new((0, -1), Rotation::East, dims),
        };

        let thruster_id = insert_part(grid_id, &mut world, &instance, true).unwrap();

        let (_mass, com) =
            get_grid_physical_props_by_id(grid_id, &world.grids, &world.parts).unwrap();
        assert_eq!(com, instance.region.grid_aligned_dims().to_meters() / 2.0);

        // obviously, turn the main thruster on
        let r = set_thruster_state(thruster_id, &mut world, true);
        assert_eq!(r, Ok(()));

        update_grid_acceleration([grid_id].into(), &mut world);

        let grid = world.grids.try_get_mut(grid_id).unwrap();

        grid.particle_location.translation = Vec2::ZERO;

        assert_eq!(grid.body_frame_forces.translation, Vec2::new(3500.0, 0.0));
        assert_eq!(grid.body_frame_forces.rotation, 0.0);
        assert_eq!(grid.parts_mass, Mass::kilograms(1000));

        // body frame acceleration should be 3.5 m/s^2
        assert_eq!(grid.linear_acceleration(), Vec2::new(3.5, 0.0));
        assert_eq!(grid.angular_acceleration(), 0.0);

        // run the simulation for 2 seconds at 50 Hz
        for _ in 0..100 {
            update_world(&mut world);
        }

        let iso = world.grids.try_get(grid_id).unwrap().particle_location;

        // this is an approximation of the following
        // continuous time kinematic equation:
        // d = 1/2 at^2  --> 0.5 * 3.5 * 2^2 = 7
        assert_eq!(iso.translation, Vec2::new(6.9299994, 0.0));
        assert_eq!(iso.rotation, 0.0);
    }

    #[test]
    fn pure_linear_acceleration_2() {
        let mut world = World::empty();

        let part_name = "test-motor";

        let thruster_data = ThrusterModel {
            model: "test-motor-model".to_string(),
            thrust: 8000.0,
            exhaust_velocity: 6000.0,
            is_rcs: false,
            throttle_rate: 0.0,
            primary_color: [0.0, 0.0, 0.0, 0.0],
            secondary_color: [0.0, 0.0, 0.0, 0.0],
            plume_length: 1.0,
            plume_angle: 0.1,
            minimum_throttle: 0.0,
            particle_scale: 1.0,
        };

        let proto = PartPrototype {
            name: part_name.to_string(),
            mass: Mass::kilograms(1000),
            dims: UVec2::new(4, 2),
            layer: PartLayer::Internal,
            excavator_data: None,
            computer_data: None,
            inventory_data: None,
            thruster_data: Some(thruster_data),
            machine_data: None,
            docking_port_data: None,
            debug_portal_data: None,
        };

        let dims = proto.dims;

        let proto_id = world.spawner.spawn();
        world.prototypes.spawn(proto_id, proto);

        let region = GridRegion::new((0, 0), Rotation::East, dims);

        let grid_id = spawn_empty_grid(&mut world, "testbed");

        let instance = PartInstance {
            name: part_name.to_string(),
            layer: PartLayer::Internal,
            region,
        };

        use find::grid_pose;

        let thruster_id = insert_part(grid_id, &mut world, &instance, true).unwrap();

        _ = set_thruster_state(thruster_id, &mut world, true);
        update_grid_acceleration([grid_id].into(), &mut world);

        let grid = world.grids.try_get_mut(grid_id).unwrap();

        grid.particle_location.translation = Vec2::ZERO;

        assert_eq!(grid.center_of_mass, Vec2::new(0.5, 0.25));

        assert_eq!(grid_pose(&world.grids, grid_id), Some(Isometry2d::ZERO));

        let expected_poses = [
            (0.000000000000, 0.000000000000, 0.000000000000),
            (0.003199999919, 0.000000000000, 0.000000000000),
            (0.009599999525, 0.000000000000, 0.000000000000),
            (0.019199999049, 0.000000000000, 0.000000000000),
            (0.031999997795, 0.000000000000, 0.000000000000),
            (0.047999996692, 0.000000000000, 0.000000000000),
            (0.067199990153, 0.000000000000, 0.000000000000),
            (0.089599989355, 0.000000000000, 0.000000000000),
            (0.115199983120, 0.000000000000, 0.000000000000),
            (0.143999978900, 0.000000000000, 0.000000000000),
            (0.175999969244, 0.000000000000, 0.000000000000),
            (0.211199969053, 0.000000000000, 0.000000000000),
            (0.249599963427, 0.000000000000, 0.000000000000),
            (0.291199952364, 0.000000000000, 0.000000000000),
            (0.335999935865, 0.000000000000, 0.000000000000),
            (0.383999943733, 0.000000000000, 0.000000000000),
            (0.435199946165, 0.000000000000, 0.000000000000),
            (0.489599943161, 0.000000000000, 0.000000000000),
            (0.547199964523, 0.000000000000, 0.000000000000),
            (0.607999980450, 0.000000000000, 0.000000000000),
            (0.671999990940, 0.000000000000, 0.000000000000),
            (0.739199995995, 0.000000000000, 0.000000000000),
            (0.809599995613, 0.000000000000, 0.000000000000),
            (0.883199989796, 0.000000000000, 0.000000000000),
            (0.959999978542, 0.000000000000, 0.000000000000),
            (1.039999961853, 0.000000000000, 0.000000000000),
            (1.123199939728, 0.000000000000, 0.000000000000),
            (1.209599971771, 0.000000000000, 0.000000000000),
            (1.299199938774, 0.000000000000, 0.000000000000),
            (1.391999959946, 0.000000000000, 0.000000000000),
            (1.487999916077, 0.000000000000, 0.000000000000),
            (1.587199926376, 0.000000000000, 0.000000000000),
            (1.689599871635, 0.000000000000, 0.000000000000),
            (1.795199871063, 0.000000000000, 0.000000000000),
            (1.903999805450, 0.000000000000, 0.000000000000),
            (2.015999794006, 0.000000000000, 0.000000000000),
            (2.131199836731, 0.000000000000, 0.000000000000),
            (2.249599695206, 0.000000000000, 0.000000000000),
            (2.371199607849, 0.000000000000, 0.000000000000),
            (2.495999574661, 0.000000000000, 0.000000000000),
            (2.623999595642, 0.000000000000, 0.000000000000),
            (2.755199432373, 0.000000000000, 0.000000000000),
            (2.889599323273, 0.000000000000, 0.000000000000),
            (3.027199268341, 0.000000000000, 0.000000000000),
            (3.167999267578, 0.000000000000, 0.000000000000),
            (3.311999320984, 0.000000000000, 0.000000000000),
            (3.459199190140, 0.000000000000, 0.000000000000),
            (3.609599113464, 0.000000000000, 0.000000000000),
            (3.763199090958, 0.000000000000, 0.000000000000),
            (3.919999122620, 0.000000000000, 0.000000000000),
            (4.079998970032, 0.000000000000, 0.000000000000),
            (4.243198871613, 0.000000000000, 0.000000000000),
            (4.409598827362, 0.000000000000, 0.000000000000),
            (4.579198837280, 0.000000000000, 0.000000000000),
            (4.751998901367, 0.000000000000, 0.000000000000),
            (4.927999019623, 0.000000000000, 0.000000000000),
            (5.107198715210, 0.000000000000, 0.000000000000),
            (5.289598464966, 0.000000000000, 0.000000000000),
            (5.475198268890, 0.000000000000, 0.000000000000),
            (5.663998126984, 0.000000000000, 0.000000000000),
            (5.855998039246, 0.000000000000, 0.000000000000),
            (6.051198005676, 0.000000000000, 0.000000000000),
            (6.249598026276, 0.000000000000, 0.000000000000),
            (6.451198101044, 0.000000000000, 0.000000000000),
            (6.655998229980, 0.000000000000, 0.000000000000),
            (6.863997936249, 0.000000000000, 0.000000000000),
            (7.075197696686, 0.000000000000, 0.000000000000),
            (7.289597511292, 0.000000000000, 0.000000000000),
            (7.507197380066, 0.000000000000, 0.000000000000),
            (7.727997303009, 0.000000000000, 0.000000000000),
            (7.951997280121, 0.000000000000, 0.000000000000),
            (8.179197311401, 0.000000000000, 0.000000000000),
            (8.409597396851, 0.000000000000, 0.000000000000),
            (8.643197059631, 0.000000000000, 0.000000000000),
            (8.879997253418, 0.000000000000, 0.000000000000),
            (9.119997024536, 0.000000000000, 0.000000000000),
            (9.363197326660, 0.000000000000, 0.000000000000),
            (9.609597206116, 0.000000000000, 0.000000000000),
            (9.859196662903, 0.000000000000, 0.000000000000),
            (10.111996650696, 0.000000000000, 0.000000000000),
            (10.367996215820, 0.000000000000, 0.000000000000),
            (10.627196311951, 0.000000000000, 0.000000000000),
            (10.889595985413, 0.000000000000, 0.000000000000),
            (11.155196189880, 0.000000000000, 0.000000000000),
            (11.423995971680, 0.000000000000, 0.000000000000),
            (11.695995330811, 0.000000000000, 0.000000000000),
            (11.971195220947, 0.000000000000, 0.000000000000),
            (12.249594688416, 0.000000000000, 0.000000000000),
            (12.531194686890, 0.000000000000, 0.000000000000),
            (12.815994262695, 0.000000000000, 0.000000000000),
            (13.103994369507, 0.000000000000, 0.000000000000),
            (13.395194053650, 0.000000000000, 0.000000000000),
            (13.689594268799, 0.000000000000, 0.000000000000),
            (13.987194061279, 0.000000000000, 0.000000000000),
            (14.287993431091, 0.000000000000, 0.000000000000),
            (14.591993331909, 0.000000000000, 0.000000000000),
            (14.899192810059, 0.000000000000, 0.000000000000),
            (15.209592819214, 0.000000000000, 0.000000000000),
            (15.523192405701, 0.000000000000, 0.000000000000),
            (15.839992523193, 0.000000000000, 0.000000000000),
        ];

        for i in 0..100 {
            let expected = expected_poses[world.ticks as usize];
            update_world(&mut world);
            let pose = find::grid_pose(&world.grids, grid_id).unwrap().to_tuple();
            assert_eq!(pose, expected, "Epoch {}", i);
            // println!("({:0.12}, {:0.12}, {:0.12}),", pose.0, pose.1, pose.2);
        }
    }

    #[test]
    fn vehicle_arrives_at_its_destination() {
        // disclaimer: this is a very fragile test, and can be affected
        // by fuel requirements, changing ship design, etc.
        // I wouldn't be surprised if I have to get rid of it.
        // But it's good for now.

        let waypoint: Isometry2d = (600.0, 800.0, 0.5).into();

        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .spawn("pollux", "fran", Isometry2d::ZERO)
            .waypoint("fran", waypoint)
            .build();

        let grid_id = find::grid_by_name(&world.grids, "fran").unwrap();

        for _ in 0..20 {
            for _ in 0..1000 {
                update_world(&mut world);
            }

            let elapsed = apparent_elapsed_time(&world);
            let pose = find::grid_pose(&world.grids, grid_id).unwrap().to_tuple();
            println!(
                "{} ({:0.1}): {}, {}, {}",
                world.ticks,
                elapsed.as_secs_f64(),
                pose.0,
                pose.1,
                pose.2
            );
        }

        assert_eq!(world.grid_acceleration_updates, 96);

        let pose = find::grid_pose(&world.grids, grid_id).unwrap();
        let error = pose.translation - waypoint.translation;

        assert!(error.x.abs() < 3.0);
        assert!(error.y.abs() < 3.0);
    }
}
