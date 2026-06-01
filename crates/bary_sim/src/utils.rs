use crate::*;
use bary_core::prelude::*;
use log::*;
use std::path::Path;

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
