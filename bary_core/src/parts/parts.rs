use crate::prelude::GridPlacement;
use crate::prelude::*;
use bevy::math::UVec2;
use bevy::prelude::*;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum PartClassification {
    Cargo,
    Machine,
    Thruster,
    Auxiliary,
    DockingPort,
    Other,
}

pub type PartDatabase = HashMap<String, PartPrototype>;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct PartPrototype {
    #[deprecated]
    pub name: String,
    pub mass: Mass,
    pub dims: UVec2,
    pub layer: Option<PartLayer>,
    pub excavator_data: Option<ExcavatorData>,
    pub computer_data: Option<ComputerData>,
    pub inventory_data: Option<InventoryData>,
    pub thruster_data: Option<ThrusterModel>,
    pub machine_data: Option<MachineData>,
    pub docking_port_data: Option<DockingPortData>,
}

impl PartPrototype {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let s = std::fs::read_to_string(path)?;
        let settings: Self = serde_yaml::from_str(&s)?;
        Ok(settings)
    }

    pub fn classification(&self) -> PartClassification {
        if self.docking_port_data.is_some() {
            PartClassification::DockingPort
        } else if self.thruster_data.is_some() {
            PartClassification::Thruster
        } else if self.machine_data.is_some() {
            PartClassification::Machine
        } else if let Some(_) = &self.inventory_data {
            PartClassification::Cargo
        } else {
            match self.layer() {
                PartLayer::Internal => PartClassification::Auxiliary,
                PartLayer::Plumbing => PartClassification::Other,
                PartLayer::Structural => PartClassification::Other,
                PartLayer::Exterior => PartClassification::Other,
            }
        }
    }
}

pub fn rotate_dims(rot: Rotation, part_meters: Vec2) -> Vec2 {
    let w = part_meters;
    match rot {
        Rotation::East | Rotation::West => Vec2::new(w.x, w.y),
        Rotation::North | Rotation::South => Vec2::new(w.y, w.x),
    }
}

impl PartPrototype {
    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn dims_meters(&self) -> Vec2 {
        self.dims().as_vec2() / GRID_CELLS_PER_METER
    }

    pub fn part_name(&self) -> &str {
        &self.name
    }

    pub fn part_mass(&self) -> Mass {
        self.mass
    }

    pub fn layer(&self) -> PartLayer {
        self.layer.unwrap_or(PartLayer::Internal)
    }

    pub fn sprites(&self) -> usize {
        1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Sequence, Hash, Deserialize, Serialize)]
pub enum PartLayer {
    Internal,
    Plumbing,
    Structural,
    Exterior,
}

impl PartLayer {
    pub fn all() -> impl Iterator<Item = PartLayer> {
        enum_iterator::all::<PartLayer>()
    }

    pub fn build_order() -> impl Iterator<Item = PartLayer> {
        [
            PartLayer::Structural,
            PartLayer::Internal,
            PartLayer::Exterior,
        ]
        .into_iter()
    }

    pub fn draw_order() -> [PartLayer; 4] {
        [
            PartLayer::Internal,
            PartLayer::Plumbing,
            PartLayer::Structural,
            PartLayer::Exterior,
        ]
    }

    pub fn to_z(self) -> u32 {
        match self {
            PartLayer::Internal => 0,
            PartLayer::Plumbing => 1,
            PartLayer::Structural => 2,
            PartLayer::Exterior => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstantiatedPart {
    pub name: String,
    layer: PartLayer,
    pub placement: GridPlacement,
}

pub fn pixel_dims_with_rotation(rot: Rotation, part: &PartPrototype) -> UVec2 {
    let dims = part.dims();
    match rot {
        Rotation::East | Rotation::West => UVec2::new(dims.x, dims.y),
        Rotation::North | Rotation::South => UVec2::new(dims.y, dims.x),
    }
}

fn meters_with_rotation(rot: Rotation, part: &PartPrototype) -> Vec2 {
    let w = part.dims_meters();
    match rot {
        Rotation::East | Rotation::West => Vec2::new(w.x, w.y),
        Rotation::North | Rotation::South => Vec2::new(w.y, w.x),
    }
}

impl InstantiatedPart {
    pub fn new(name: impl Into<String>, layer: PartLayer, placement: GridPlacement) -> Self {
        Self {
            name: name.into(),
            layer,
            placement,
        }
    }

    pub fn from_prototype(proto: PartPrototype, pos: PartCoord, rot: Rotation) -> Self {
        let dims = proto.dims();

        let placement = GridPlacement::new(pos, rot, dims);

        Self {
            name: proto.name.to_string(),
            layer: proto.layer(),
            placement,
        }
    }

    pub fn layer(&self) -> PartLayer {
        self.layer
    }

    pub fn dims_grid(&self) -> UVec2 {
        self.placement.grid_aligned_dims().inner().as_uvec2()
    }

    pub fn dims_meters(&self) -> Vec2 {
        self.placement.grid_aligned_dims().to_meters()
    }

    pub fn center_meters(&self) -> Vec2 {
        let iso = self.placement.center_isometry();
        iso.translation
    }

    pub fn origin(&self) -> PartCoord {
        self.placement.bottom_left()
    }

    pub fn origin_meters(&self) -> Vec2 {
        self.placement.bottom_left().to_meters()
    }

    pub fn set_origin(&mut self, p: IVec2) {
        self.placement.set_bottom_left(p.into());
    }

    pub fn with_origin(&self, p: IVec2) -> Self {
        let mut ret = self.clone();
        ret.set_origin(p);
        ret
    }

    pub fn obb(&self, angle: f32, scale: f32, pos: Vec2) -> OBB {
        let dims = self.dims_meters();
        let center = rotate(self.origin().to_meters() + dims / 2.0, angle) * scale;
        OBB::new(
            AABB::from_arbitrary(scale * -dims / 2.0, scale * dims / 2.0),
            angle,
        )
        .offset(center + pos)
    }

    pub fn rotation(&self) -> Rotation {
        self.placement.rot()
    }

    pub fn set_rotation(&mut self, rot: Rotation) {
        self.placement.set_rot(rot);
    }

    pub fn upper_left(&self) -> PartCoord {
        let dims = self.dims_grid();
        self.origin() + PartCoord::new(IVec2::Y * dims.y as i32)
    }

    pub fn rotate_ccw(&mut self) {
        let upper_left = self.upper_left();
        let new_origin = IVec2::Y.rotate(upper_left.inner());
        self.set_rotation(enum_iterator::next_cycle(&self.rotation()));
        self.set_origin(new_origin);
    }

    pub fn machine_data<'a>(proto: &'a PartPrototype) -> Option<&'a MachineData> {
        proto.machine_data.as_ref()
    }

    pub fn inventory_data<'a>(proto: &'a PartPrototype) -> Option<&'a InventoryData> {
        proto.inventory_data.as_ref()
    }

    pub fn computer_data<'a>(proto: &'a PartPrototype) -> Option<&'a ComputerData> {
        proto.computer_data.as_ref()
    }

    pub fn docking_port_data<'a>(proto: &'a PartPrototype) -> Option<&'a DockingPortData> {
        proto.docking_port_data.as_ref()
    }

    pub fn excavator_data<'a>(proto: &'a PartPrototype) -> Option<&'a ExcavatorData> {
        proto.excavator_data.as_ref()
    }

    pub fn thruster_data<'a>(proto: &'a PartPrototype) -> Option<&'a ThrusterModel> {
        proto.thruster_data.as_ref()
    }
}
