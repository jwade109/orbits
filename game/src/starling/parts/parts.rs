use crate::starling::math::*;
use crate::starling::parts::*;
use bevy::math::UVec2;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum PartClassification {
    Cargo,
    Machine,
    Thruster,
    Auxiliary,
    DockingPort,
    Other,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

    pub fn sprite_path(&self) -> &str {
        self.part_name()
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
    pub pos: PartCoord,
    pub rot: Rotation,
    pub dims: UVec2,
    pub proto: PartPrototype,
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
    pub fn from_prototype(proto: PartPrototype, pos: PartCoord, rot: Rotation) -> Self {
        let dims = proto.dims();

        Self {
            pos,
            rot,
            dims,
            proto,
        }
    }

    pub fn prototype(&self) -> PartPrototype {
        self.proto.clone()
    }

    pub fn layer(&self) -> PartLayer {
        self.prototype().layer()
    }

    pub fn total_mass(&self) -> Mass {
        self.proto.mass
    }

    pub fn dims_grid(&self) -> UVec2 {
        pixel_dims_with_rotation(self.rot, &self.prototype())
    }

    pub fn dims_meters(&self) -> Vec2 {
        meters_with_rotation(self.rot, &self.prototype())
    }

    pub fn center_meters(&self) -> Vec2 {
        let dims = rotate_dims(self.rot, self.dims.as_vec2() / GRID_CELLS_PER_METER);
        let origin = self.pos.to_meters();
        origin + dims / 2.0
    }

    pub fn origin(&self) -> PartCoord {
        self.pos
    }

    pub fn origin_meters(&self) -> Vec2 {
        self.pos.to_meters()
    }

    pub fn set_origin(&mut self, p: IVec2) {
        self.pos.0 = p;
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
        self.rot
    }

    pub fn set_rotation(&mut self, rot: Rotation) {
        self.rot = rot;
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

    pub fn machine_data(&self) -> Option<&MachineData> {
        self.proto.machine_data.as_ref()
    }

    pub fn inventory_data(&self) -> Option<&InventoryData> {
        self.proto.inventory_data.as_ref()
    }

    pub fn computer_data(&self) -> Option<&ComputerData> {
        self.proto.computer_data.as_ref()
    }

    pub fn docking_port_data(&self) -> Option<&DockingPortData> {
        self.proto.docking_port_data.as_ref()
    }

    pub fn excavator_data(&self) -> Option<&ExcavatorData> {
        self.proto.excavator_data.as_ref()
    }

    pub fn thruster_data(&self) -> Option<&ThrusterModel> {
        self.proto.thruster_data.as_ref()
    }
}
