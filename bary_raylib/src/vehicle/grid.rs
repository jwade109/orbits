use crate::ops;
use crate::vehicle::Part;
use crate::{components::Components, ops::update_grid_physical_props_by_id};
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{result::BaryResult, world::World};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct PartOccupancy {
    pub internal: Option<Ent>,
    pub structural: Option<Ent>,
    pub external: Option<Ent>,
    pub plumbing: Option<Ent>,
}

fn clear_if_equal(val: &mut Option<Ent>, other: Ent) {
    if *val == Some(other) {
        *val = None;
    }
}

impl PartOccupancy {
    pub const EMPTY: Self = Self::new();

    pub const fn new() -> Self {
        Self {
            internal: None,
            structural: None,
            external: None,
            plumbing: None,
        }
    }

    // TODO(testing) especially with to_array
    pub fn top(&self) -> Option<Ent> {
        let arr: Vec<_> = self.to_array().iter().filter_map(|e| *e).collect();
        arr.last().map(|e| *e)
    }

    pub fn has_any(&self) -> bool {
        self.internal.is_some()
            || self.structural.is_some()
            || self.external.is_some()
            || self.plumbing.is_some()
    }

    pub fn to_array(&self) -> [Option<Ent>; 4] {
        [self.internal, self.structural, self.external, self.plumbing]
    }

    pub fn iter(&self) -> impl Iterator<Item = (PartLayer, Ent)> + use<'_> {
        [
            self.internal.map(|e| (PartLayer::Internal, e)),
            self.structural.map(|e| (PartLayer::Structural, e)),
            self.external.map(|e| (PartLayer::Exterior, e)),
            self.plumbing.map(|e| (PartLayer::Plumbing, e)),
        ]
        .into_iter()
        .filter_map(|e| e)
    }

    pub fn mark_internal(&mut self, id: impl Into<Option<Ent>>) {
        self.internal = id.into();
    }

    pub fn mark_structural(&mut self, id: impl Into<Option<Ent>>) {
        self.structural = id.into();
    }

    pub fn mark_external(&mut self, id: impl Into<Option<Ent>>) {
        self.external = id.into();
    }

    pub fn mark_plumbing(&mut self, id: impl Into<Option<Ent>>) {
        self.plumbing = id.into();
    }

    pub fn mark(&mut self, layer: PartLayer, id: impl Into<Option<Ent>>) {
        match layer {
            PartLayer::Internal => self.mark_internal(id),
            PartLayer::Structural => self.mark_structural(id),
            PartLayer::Exterior => self.mark_external(id),
            PartLayer::Plumbing => self.mark_plumbing(id),
        }
    }

    pub fn remove(&mut self, part_id: Ent) {
        clear_if_equal(&mut self.internal, part_id);
        clear_if_equal(&mut self.structural, part_id);
        clear_if_equal(&mut self.external, part_id);
        clear_if_equal(&mut self.plumbing, part_id);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VehicleGrid {
    pub name: String,
    pub parts_mass: Mass,
    pub center_of_mass: Vec2,
    pub pose: Isometry2d,
    pub velocity: Isometry2d,
    pub body_frame_forces: Isometry2d,
    pub parts: BTreeSet<Ent>,
    pub thrusters: BTreeSet<Ent>,
    pub computers: BTreeSet<Ent>,
    pub lights: BTreeSet<Ent>,
    /// Lower bound is inclusive; upper is exclusive.
    /// Extent is upper minus lower. An empty grid
    /// will have zero extent.
    pub bounds: (IVec2, IVec2),

    // TODO this is not tested in any way.
    #[serde(skip)]
    pub occupancy: BTreeMap<(i32, i32), PartOccupancy>,
}

impl VehicleGrid {
    pub fn empty() -> Self {
        Self::with_name("")
    }

    pub fn with_name(name: impl Into<String>) -> Self {
        VehicleGrid {
            name: name.into(),
            parts_mass: Mass::ZERO,
            center_of_mass: Vec2::ZERO,
            pose: Isometry2d::ZERO,
            velocity: Isometry2d::ZERO,
            body_frame_forces: Isometry2d::ZERO,
            parts: BTreeSet::new(),
            thrusters: BTreeSet::new(),
            computers: BTreeSet::new(),
            lights: BTreeSet::new(),
            bounds: (IVec2::ZERO, IVec2::ZERO),
            occupancy: BTreeMap::new(),
        }
    }

    pub fn parts_mass(&self) -> Mass {
        self.parts_mass
    }

    pub fn linear_acceleration(&self) -> Vec2 {
        self.body_frame_forces.translation / self.parts_mass.to_kg_f64() as f32
    }

    pub fn angular_acceleration(&self) -> f32 {
        let moment_of_inertia = self.parts_mass.to_kg_f64() as f32 * 100.0;
        self.body_frame_forces.rotation / moment_of_inertia
    }

    /// TODO test this.
    pub fn get_parts_at(&self, p: PartCoord) -> Option<&PartOccupancy> {
        let p = (p.0.x, p.0.y);
        self.occupancy.get(&p)
    }

    fn update_bounds(&mut self) {
        let mut bounds: Option<(IVec2, IVec2)> = None;
        for cell in self.occupancy.keys() {
            let lower = IVec2::new(cell.0, cell.1);
            let upper = lower + IVec2::ONE;
            if let Some(c) = &mut bounds {
                c.0.x = c.0.x.min(lower.x);
                c.0.y = c.0.y.min(lower.y);
                c.1.x = c.1.x.max(upper.x);
                c.1.y = c.1.y.max(upper.y);
            } else {
                bounds = Some((lower, upper));
            }
        }
        self.bounds = bounds.unwrap_or((IVec2::ZERO, IVec2::ZERO));
    }

    pub fn mark_occupied(&mut self, placement: GridPlacement, layer: PartLayer, id: Ent) {
        for cell in placement.cells() {
            let key = (cell.0.x, cell.0.y);
            self.occupancy
                .entry(key)
                .and_modify(|e| {
                    e.mark(layer, id);
                })
                .or_insert({
                    let mut occ = PartOccupancy::default();
                    occ.mark(layer, id);
                    occ
                });
        }
        self.update_bounds();
    }

    pub fn remove_from_index(&mut self, id: Ent) {
        for occ in self.occupancy.values_mut() {
            occ.remove(id);
        }

        self.occupancy.retain(|_, e| e.has_any());

        self.parts.retain(|e| *e != id);
        self.thrusters.retain(|e| *e != id);
        self.computers.retain(|e| *e != id);
        self.lights.retain(|e| *e != id);
        self.update_bounds();
    }

    pub fn calculate_islands(&self) -> Vec<BTreeSet<Ent>> {
        let mut unvisited = BTreeSet::new();

        for key in self.occupancy.keys() {
            unvisited.insert(*key);
        }

        let get_neighbors = |(x, y): (i32, i32)| [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)];

        let mut ret = Vec::new();

        while let Some(seed) = unvisited.first().cloned() {
            let mut parts = BTreeSet::new();

            let mut open_set: BTreeSet<(i32, i32)> = [seed].into();
            let mut closed_set = BTreeSet::new();
            while let Some(current) = open_set.pop_first() {
                if closed_set.contains(&current) {
                    continue;
                }
                closed_set.insert(current);
                if !unvisited.contains(&current) {
                    continue;
                };
                unvisited.remove(&current);

                if let Some(occ) = self.occupancy.get(&current) {
                    for (_, id) in occ.iter() {
                        parts.insert(id);
                    }
                }

                for n in get_neighbors(current) {
                    open_set.insert(n);
                }
            }

            ret.push(parts);
        }

        ret.sort_by_key(|e| -(e.len() as i32));

        ret
    }
}

/// Atomically moves an entity ID from src to dst, if it exists in src.
/// Returns true if the ID was moved.
pub fn move_index(src: &mut BTreeSet<Ent>, dst: &mut BTreeSet<Ent>, id: Ent) -> bool {
    if src.contains(&id) {
        src.remove(&id);
        dst.insert(id);
        true
    } else {
        false
    }
}

/// Moves a part from the source grid to the destination grid, removing
/// and adding to the relevant indices.
/// The mass of both grids will need to be updated separately.
pub fn move_part(
    src: &mut VehicleGrid,
    dst: &mut VehicleGrid,
    part_id: Ent,
    new_placement: GridPlacement,
    layer: PartLayer,
) -> bool {
    let success = move_index(&mut src.parts, &mut dst.parts, part_id);
    move_index(&mut src.thrusters, &mut dst.thrusters, part_id);
    move_index(&mut src.computers, &mut dst.computers, part_id);
    move_index(&mut src.lights, &mut dst.lights, part_id);
    move_index(&mut src.parts, &mut dst.parts, part_id);

    if success {
        src.remove_from_index(part_id);
        dst.mark_occupied(new_placement, layer, part_id);
    }

    success
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
pub fn remove_part_without_integrity_check(
    world: &mut World,
    part_id: Ent,
    update_props: bool,
) -> BaryResult<PartInstance> {
    world.chat.log(format!("Removing part {}", part_id));
    let part = world.parts.try_get(part_id)?;
    let grid_id = part.grid_id;
    let proto = world.prototypes.try_get(part.prototype)?;
    let grid = world.grids.try_get_mut(grid_id)?;
    let name = grid.name.clone();

    let instance = PartInstance::new(&proto.name, proto.layer, part.placement);

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
        world.chat.log(format!("Deleted empty grid \"{}\"", name));
    }

    if update_props {
        update_grid_physical_props_by_id(grid_id, &mut world.grids, &world.parts);
    }

    Ok(instance)
}

pub fn duplicate_part_to_new_grid(world: &mut World, part_id: Ent) -> BaryResult<Ent> {
    let part = world.parts.try_get(part_id)?;
    let grid = world.grids.try_get(part.grid_id)?;
    let new_name = format!("{}-debris", grid.name);
    let new_part_pose = grid.pose * part.placement.origin_isometry();
    let new_grid_vel = grid.velocity + Isometry2d::new(randvec(1.0, 3.0), rand(-0.1, 0.1));
    let proto = world.prototypes.try_get(part.prototype)?;
    let instance = PartInstance {
        name: proto.name.clone(),
        layer: proto.layer,
        placement: GridPlacement::new((0, 0), Rotation::East, proto.dims),
    };
    let new_grid_id = ops::spawn_empty_grid(world, new_name);
    ops::set_grid_pose(world, new_grid_id, new_part_pose)?;
    ops::set_grid_vel(world, new_grid_id, new_grid_vel)?;
    let new_part_id = ops::insert_part(new_grid_id, world, &instance, true)?;
    Ok(new_part_id)
}

pub fn detach_part_from_parent(world: &mut World, part_id: Ent) -> BaryResult<Ent> {
    let part = world.parts.try_get(part_id)?;
    let grid_id = part.grid_id;
    let new_part_id = duplicate_part_to_new_grid(world, part_id)?;
    remove_part_without_integrity_check(world, part_id, true)?;
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

    let mut ids = Vec::new();
    let rebuilt = rebuild_index_from_islands(grid, &islands, &world.parts)?;

    for (i, mut r) in rebuilt.into_iter().enumerate() {
        r.velocity.translation += randvec(0.1, 0.2);
        r.velocity.rotation += rand(-0.05, 0.05);
        if i == 0 {
            world.grids.insert(grid_id, r);
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
        update_grid_physical_props_by_id(*id, &mut world.grids, &world.parts)?;
    }

    Ok(ids)
}

pub fn rebuild_index_from_island(
    src: &VehicleGrid,
    island: &BTreeSet<Ent>,
    parts: &Components<Part>,
) -> BaryResult<VehicleGrid> {
    let mut dst = VehicleGrid::with_name("from_island");
    dst.pose = src.pose;
    dst.velocity = src.velocity;
    for part_id in island.iter().map(|i| *i) {
        let part = parts.try_get(part_id)?;
        dst.parts_mass += part.mass;
        dst.parts.insert(part_id);
        dst.mark_occupied(part.placement, part.layer, part_id);
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
        .map(|island| rebuild_index_from_island(src, island, parts))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query;
    use crate::result::BaryError;
    use crate::tests::assert_world_is_consistent;
    use crate::world_builder::WorldBuilder;

    #[test]
    fn rebuilding_grid_from_islands() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .spawn("pollux", (0.0, 0.0, 0.0))
            .build();

        let grid_id = query::grid_by_name(&world.grids, "pollux").unwrap();
        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts_mass, Mass::grams(35134000));
        assert_eq!(grid.parts.len(), 98);

        // slice a thing down the middle of the ship
        let mut parts = BTreeSet::new();
        let x = 0;
        for y in -10..=10 {
            if let Some(occ) = grid.occupancy.get(&(x, y)) {
                for (_, id) in occ.iter() {
                    parts.insert(id);
                }
            }
        }

        for part_id in parts {
            let r = remove_part_without_integrity_check(&mut world, part_id, true);
            assert!(r.is_ok());
        }

        let mut grid = world.grids.try_get(grid_id).unwrap().clone();
        let islands = grid.calculate_islands();

        assert_eq!(islands.len(), 2);

        let rebuilt = rebuild_index_from_islands(&mut grid, &islands, &world.parts).unwrap();

        assert_eq!(rebuilt.len(), 2);

        let ra = &rebuilt[0];
        let rb = &rebuilt[1];

        assert_eq!(ra.parts.len(), 45);
        assert_eq!(rb.parts.len(), 40);

        assert_eq!(ra.thrusters.len(), 9);
        assert_eq!(rb.thrusters.len(), 9);

        assert_eq!(ra.computers.len(), 0);
        assert_eq!(rb.computers.len(), 1);

        assert_eq!(ra.lights.len(), 6);
        assert_eq!(rb.lights.len(), 6);

        assert_eq!(ra.parts_mass, Mass::grams(16817000));
        assert_eq!(rb.parts_mass, Mass::grams(14797000));
    }

    #[test]
    fn removing_parts_from_grid() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .spawn("pollux", (0.0, 0.0, 0.0))
            .build();

        let grid_id = query::grid_by_name(&world.grids, "pollux").unwrap();
        let grid = world.grids.try_get(grid_id).unwrap();
        let parts: Vec<_> = grid.parts.iter().collect();

        assert_eq!(grid.parts_mass, Mass::grams(35134000));
        assert_eq!(grid.parts.len(), 98);

        let part_a = *parts[12];
        let part_b = *parts[20];
        let part_c = *parts[37];

        assert_eq!(part_a, Ent(44));
        assert_eq!(part_b, Ent(52));
        assert_eq!(part_c, Ent(69));

        let op_a = remove_part_without_integrity_check(&mut world, part_a, false);
        let op_b = remove_part_without_integrity_check(&mut world, part_b, false);
        let op_c = remove_part_without_integrity_check(&mut world, part_c, true);

        let placement_a = GridPlacement::new((-12, -9), Rotation::North, (1, 1));
        let placement_b = GridPlacement::new((10, -5), Rotation::South, (2, 2));
        let placement_c = GridPlacement::new((10, 3), Rotation::South, (2, 2));

        let part_a = PartInstance::new("rcs", PartLayer::Internal, placement_a);
        let part_b = PartInstance::new("plate", PartLayer::Exterior, placement_b);
        let part_c = PartInstance::new("plate", PartLayer::Exterior, placement_c);

        assert_eq!(op_a, Ok(part_a));
        assert_eq!(op_b, Ok(part_b));
        assert_eq!(op_c, Ok(part_c));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts_mass, Mass::grams(35073000));
        assert_eq!(grid.parts.len(), 95);

        assert_world_is_consistent(&world);
    }

    #[test]
    fn split_grid_into_two_grids() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .spawn("pollux", (0.0, 0.0, 0.0))
            .build();

        let grid_id = query::grid_by_name(&world.grids, "pollux").unwrap();

        // this should fail if the grid ID is bad, obviously.
        let result = split_grid_if_necessary(&mut world, Ent(0));

        assert_eq!(result, Err(BaryError::EntityNotFound(Ent(0))));

        let result = split_grid_if_necessary(&mut world, grid_id);

        assert_eq!(result, Ok(vec![]));

        let grid = world.grids.try_get(grid_id).unwrap();
        assert_eq!(grid.parts_mass, Mass::grams(35134000));
        assert_eq!(grid.parts.len(), 98);

        // slice a thing down the middle of the ship
        let mut parts = BTreeSet::new();
        let x = 2;
        for y in -10..=10 {
            if let Some(occ) = grid.occupancy.get(&(x, y)) {
                for (_, id) in occ.iter() {
                    parts.insert(id);
                }
            }
        }

        assert_eq!(parts.len(), 8);

        for part_id in parts {
            let r = remove_part_without_integrity_check(&mut world, part_id, true);
            assert!(r.is_ok());
        }

        let result = split_grid_if_necessary(&mut world, grid_id);

        assert_eq!(result, Ok(vec![Ent(31), Ent(130)]));

        let grid = world.grids.try_get(Ent(31)).unwrap();
        assert_eq!(grid.parts_mass, Mass::grams(17757000));
        assert_eq!(grid.parts.len(), 52);

        let grid = world.grids.try_get(Ent(130)).unwrap();
        assert_eq!(grid.parts_mass, Mass::grams(14247000));
        assert_eq!(grid.parts.len(), 38);

        assert_world_is_consistent(&world);
    }
}
