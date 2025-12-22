use crate::starling::aabb::*;
use crate::starling::factory::Mass;
use crate::starling::math::*;
use crate::starling::parts::*;
use bevy::math::UVec2;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use std::path::Path;

// TODO reduce scope of this constant
pub const PIXELS_PER_METER: f32 = 20.0;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PartPrototype {
    #[deprecated]
    name: String,
    mass: Mass,
    dims: UVec2,
    layer: Option<PartLayer>,
    excavator_data: Option<ExcavatorData>,
    computer_data: Option<ComputerData>,
    inventory_data: Option<InventoryData>,
    thruster_data: Option<ThrusterModel>,
    machine_data: Option<MachineData>,
    docking_port_data: Option<DockingPortData>,
}

impl PartPrototype {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let s = std::fs::read_to_string(path)?;
        let settings: Self = serde_yaml::from_str(&s)?;
        Ok(settings)
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
        self.dims().as_vec2() / PIXELS_PER_METER
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

    pub fn draw_order() -> [PartLayer; 3] {
        [
            PartLayer::Internal,
            PartLayer::Structural,
            PartLayer::Exterior,
        ]
    }

    pub fn to_z(self) -> u32 {
        match self {
            PartLayer::Internal => 0,
            PartLayer::Structural => 2,
            PartLayer::Exterior => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstantiatedPart {
    builds_performed: u32,
    builds_required: u32,
    pos: IVec2,
    rot: Rotation,
    dims: UVec2,
    proto: PartPrototype,
}

pub fn pixel_dims_with_rotation(rot: Rotation, part: &PartPrototype) -> UVec2 {
    let dims = part.dims();
    match rot {
        Rotation::East | Rotation::West => UVec2::new(dims.x, dims.y),
        Rotation::North | Rotation::South => UVec2::new(dims.y, dims.x),
    }
}

pub fn meters_with_rotation(rot: Rotation, part: &PartPrototype) -> Vec2 {
    let w = part.dims_meters();
    match rot {
        Rotation::East | Rotation::West => Vec2::new(w.x, w.y),
        Rotation::North | Rotation::South => Vec2::new(w.y, w.x),
    }
}

impl InstantiatedPart {
    pub fn from_prototype(proto: PartPrototype, pos: IVec2, rot: Rotation) -> Self {
        let dims = proto.dims();

        Self {
            builds_performed: 0,
            builds_required: (dims.x * dims.y).clamp(30, 2000),
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

    pub fn build(&mut self) {
        if self.builds_performed < self.builds_required {
            self.builds_performed += 1;
        }
    }

    pub fn build_all(&mut self) {
        self.builds_performed = self.builds_required;
    }

    pub fn percent_built(&self) -> f32 {
        (self.builds_performed as f32 / self.builds_required as f32).clamp(0.0, 1.0)
    }

    pub fn is_built(&self) -> bool {
        self.builds_performed == self.builds_required
    }

    pub fn dims_grid(&self) -> UVec2 {
        pixel_dims_with_rotation(self.rot, &self.prototype())
    }

    pub fn dims_meters(&self) -> Vec2 {
        meters_with_rotation(self.rot, &self.prototype())
    }

    pub fn center_meters(&self) -> Vec2 {
        let dims = rotate_dims(self.rot, self.dims.as_vec2() / PIXELS_PER_METER);
        let origin = self.pos.as_vec2() / PIXELS_PER_METER;
        origin + dims / 2.0
    }

    pub fn origin(&self) -> IVec2 {
        self.pos
    }

    pub fn origin_meters(&self) -> Vec2 {
        self.pos.as_vec2() / PIXELS_PER_METER
    }

    pub fn set_origin(&mut self, p: IVec2) {
        self.pos = p;
    }

    pub fn with_origin(&self, p: IVec2) -> Self {
        let mut ret = self.clone();
        ret.set_origin(p);
        ret
    }

    pub fn obb(&self, angle: f32, scale: f32, pos: Vec2) -> OBB {
        let dims = self.dims_meters();
        let center = rotate(
            self.origin().as_vec2() / crate::starling::prelude::PIXELS_PER_METER + dims / 2.0,
            angle,
        ) * scale;
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

    pub fn rotated(&self) -> Self {
        let mut ret = self.clone();
        let old_half_dims = ret.dims_grid().as_vec2() / 2.0;
        let old_center = ret.origin().as_vec2() + old_half_dims;
        let new_center = rotate(old_center, PI / 2.0);
        ret.set_rotation(enum_iterator::next_cycle(&ret.rotation()));
        let new_half_dims = ret.dims_grid().as_vec2() / 2.0;
        let new_corner = new_center - new_half_dims;
        ret.set_origin(vround(new_corner));
        ret
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
