use bary_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub struct PartOccupancy {
    internal: Option<Ent>,
    structural: Option<Ent>,
    external: Option<Ent>,
    plumbing: Option<Ent>,
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

    pub fn iter(&self) -> impl Iterator<Item = Ent> + use<'_> {
        [self.internal, self.structural, self.external, self.plumbing]
            .into_iter()
            .filter_map(|e| e)
    }

    pub fn mark_internal(&mut self, id: impl Into<Option<Ent>>) {
        self.internal = id.into();
    }

    pub fn mark_structural(&mut self, id: impl Into<Option<Ent>>) {
        self.internal = id.into();
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
            PartLayer::Structural => self.mark_internal(id),
            PartLayer::Exterior => self.mark_external(id),
            PartLayer::Plumbing => self.mark_plumbing(id),
        }
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
            pose: Isometry2d::IDENTITY,
            velocity: Isometry2d::IDENTITY,
            body_frame_forces: Isometry2d::IDENTITY,
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
}
