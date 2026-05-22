use crate::sim::*;
use bary_core::prelude::*;
use bary_parts::*;
use bary_sim::*;
use log::info;
use std::collections::{BTreeMap, BTreeSet};

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
