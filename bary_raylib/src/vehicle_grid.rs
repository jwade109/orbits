use bary_core::prelude::*;
use log::*;
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
    pub fn with_name(name: impl Into<String>) -> Self {
        VehicleGrid {
            name: name.into(),
            parts_mass: Mass::ZERO,
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

    pub fn assess_integrity(&self) -> Vec<BTreeSet<Ent>> {
        let mut unvisited = BTreeSet::new();

        for key in self.occupancy.keys() {
            unvisited.insert(*key);
        }

        let get_neighbors = |(x, y): (i32, i32)| [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)];

        let mut ret = Vec::new();

        while let Some(seed) = unvisited.first().cloned() {
            let mut parts = BTreeSet::new();

            let mut open_set: BTreeSet<(i32, i32)> = [seed].into();
            let mut count = 0;
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

                count += 1;

                for n in get_neighbors(current) {
                    open_set.insert(n);
                }
            }

            warn!("seed: {:?}, cells: {}, parts: {}", seed, count, parts.len());

            ret.push(parts);
        }

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
/// TODO(bug) It also doesn't update the grid's acceleration, so if a thruster
/// is removed while it's firing that acceleration will remain until the grid's
/// acceleration is recalculated.
/// Returns the grid which was modified, if any.
// TODO(testing) very, VERY important to test!
pub fn remove_part_without_integrity_check(world: &mut World, part_id: Ent) -> BaryResult<Ent> {
    world.chat.log(format!("Removing part {}", part_id));
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

    if grid.parts.is_empty() {
        world.grids.despawn(grid_id)?;
        world.chat.log(format!("Deleted empty grid \"{}\"", name));
    }

    Ok(grid_id)
}

pub fn split_grid_if_necessary_todo_implement_me(world: &World, grid_id: Ent) -> BaryResult<usize> {
    let grid = world.grids.try_get(grid_id)?;
    let groups = grid.assess_integrity();
    Ok(groups.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::BaryError;
    use crate::systems::find;
    use crate::tests::assert_world_is_consistent;
    use crate::world_builder::WorldBuilder;

    #[test]
    fn removing_parts_from_grid() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .spawn("pollux", (0.0, 0.0, 0.0))
            .build();

        let grid_id = find::grid_by_name(&world.grids, "pollux").unwrap();
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

        let op_a = remove_part_without_integrity_check(&mut world, part_a);
        let op_b = remove_part_without_integrity_check(&mut world, part_b);
        let op_c = remove_part_without_integrity_check(&mut world, part_c);

        assert_eq!(op_a, Ok(grid_id));
        assert_eq!(op_b, Ok(grid_id));
        assert_eq!(op_c, Ok(grid_id));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts_mass, Mass::grams(35073000));
        assert_eq!(grid.parts.len(), 95);
    }

    #[test]
    fn split_grid_into_two_grids() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .spawn("pollux", (0.0, 0.0, 0.0))
            .build();

        let grid_id = find::grid_by_name(&world.grids, "pollux").unwrap();

        // this should fail if the grid ID is bad, obviously.
        let result = split_grid_if_necessary_todo_implement_me(&world, Ent(0));

        assert_eq!(result, Err(BaryError::EntityNotFound));

        let result = split_grid_if_necessary_todo_implement_me(&world, grid_id);

        assert_eq!(result, Ok(1));

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
            let r = remove_part_without_integrity_check(&mut world, part_id);
            assert_eq!(r, Ok(grid_id));
        }

        let grid = world.grids.try_get(grid_id).unwrap();
        assert_eq!(grid.parts_mass, Mass::grams(32004000));
        assert_eq!(grid.parts.len(), 90);

        let result = split_grid_if_necessary_todo_implement_me(&world, grid_id);

        assert_eq!(result, Ok(2));

        // another slice
        let mut parts = BTreeSet::new();
        for i in 0..5 {
            if let Some(occ) = grid.occupancy.get(&(-i, i)) {
                for (_, id) in occ.iter() {
                    parts.insert(id);
                }
            }
        }

        assert_eq!(parts.len(), 5);

        for part_id in parts {
            let r = remove_part_without_integrity_check(&mut world, part_id);
            assert_eq!(r, Ok(grid_id));
        }

        let grid = world.grids.try_get(grid_id).unwrap();
        assert_eq!(grid.parts_mass, Mass::grams(30239000));
        assert_eq!(grid.parts.len(), 85);

        let result = split_grid_if_necessary_todo_implement_me(&world, grid_id);

        assert_eq!(result, Ok(3));

        assert_world_is_consistent(&world);
    }
}
