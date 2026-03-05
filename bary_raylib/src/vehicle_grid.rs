use bary_core::prelude::*;
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
    pub moment_of_inertia: f32,
    pub pose: Isometry2d,
    pub velocity: Isometry2d,
    pub body_frame_forces: Isometry2d,
    pub parts: Vec<Ent>,
    pub thrusters: Vec<Ent>,
    pub computers: Vec<Ent>,
    pub lights: Vec<Ent>,

    // TODO this is not tested in any way.
    #[serde(skip)]
    pub occupancy: BTreeMap<(i32, i32), PartOccupancy>,
}

impl VehicleGrid {
    pub fn with_name(name: impl Into<String>) -> Self {
        VehicleGrid {
            name: name.into(),
            parts_mass: Mass::ZERO,
            moment_of_inertia: 0.0,
            pose: Isometry2d::ZERO,
            velocity: Isometry2d::ZERO,
            body_frame_forces: Isometry2d::ZERO,
            parts: Vec::new(),
            thrusters: Vec::new(),
            computers: Vec::new(),
            lights: Vec::new(),
            occupancy: BTreeMap::new(),
        }
    }

    pub fn linear_acceleration(&self) -> Vec2 {
        self.body_frame_forces.translation / self.parts_mass.to_kg_f64() as f32
    }

    pub fn angular_acceleration(&self) -> f32 {
        self.body_frame_forces.rotation / self.moment_of_inertia
    }

    /// TODO test this.
    pub fn get_parts_at(&self, p: PartCoord) -> Option<&PartOccupancy> {
        let p = (p.0.x, p.0.y);
        self.occupancy.get(&p)
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
    }

    pub fn assess_integrity(&self) {
        let mut unvisited = BTreeSet::new();

        for key in self.occupancy.keys() {
            unvisited.insert(*key);
        }

        let get_neighbors = |(x, y): (i32, i32)| [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)];

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
        }
    }
}
