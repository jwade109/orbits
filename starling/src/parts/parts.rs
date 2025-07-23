use crate::factory::Mass;
use crate::math::*;
use crate::parts::*;
use enum_iterator::Sequence;
use crate::aabb::*;
use serde::{Deserialize, Serialize};

// TODO reduce scope of this constant
pub const PIXELS_PER_METER: f32 = 20.0;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PartPrototype {
    Thruster(ThrusterModel),
    Tank(TankModel),
    Radar(Radar),
    Cargo(Cargo),
    Magnetorquer(Magnetorquer),
    Machine(Machine),
    Generic(Generic),
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
        match self {
            Self::Thruster(p) => p.dims(),
            Self::Tank(p) => p.dims(),
            Self::Radar(p) => p.dims(),
            Self::Cargo(p) => p.dims(),
            Self::Magnetorquer(p) => p.dims(),
            Self::Generic(p) => p.dims(),
            Self::Machine(p) => p.dims(),
        }
    }

    pub fn dims_meters(&self) -> Vec2 {
        self.dims().as_vec2() / PIXELS_PER_METER
    }

    pub fn part_name(&self) -> &str {
        match self {
            Self::Thruster(p) => p.part_name(),
            Self::Tank(p) => p.part_name(),
            Self::Radar(p) => p.part_name(),
            Self::Cargo(p) => p.part_name(),
            Self::Magnetorquer(p) => p.part_name(),
            Self::Generic(p) => p.part_name(),
            Self::Machine(p) => p.part_name(),
        }
    }

    pub fn dry_mass(&self) -> Mass {
        match self {
            Self::Thruster(p) => p.mass(),
            Self::Tank(p) => p.dry_mass(),
            Self::Radar(p) => p.mass(),
            Self::Cargo(p) => p.empty_mass(),
            Self::Magnetorquer(p) => p.mass(),
            Self::Generic(p) => p.mass(),
            Self::Machine(p) => p.mass(),
        }
    }

    pub fn layer(&self) -> PartLayer {
        match self {
            Self::Thruster(..) => PartLayer::Internal,
            Self::Tank(..) => PartLayer::Internal,
            Self::Radar(..) => PartLayer::Internal,
            Self::Cargo(..) => PartLayer::Internal,
            Self::Magnetorquer(..) => PartLayer::Internal,
            Self::Generic(p) => p.layer(),
            Self::Machine(..) => PartLayer::Internal,
        }
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
            PartLayer::Plumbing,
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
}

#[derive(Debug, Clone)]
pub enum InstantiatedPartVariant {
    Thruster(ThrusterModel, ThrusterInstanceData),
    Tank(TankModel, TankInstanceData),
    Radar(Radar),
    Cargo(Cargo, CargoInstanceData),
    Magnetorquer(Magnetorquer, MagnetorquerInstanceData),
    Machine(Machine, MachineInstanceData),
    Generic(Generic),
}

#[derive(Debug, Clone)]
pub struct InstantiatedPart {
    builds_performed: u32,
    builds_required: u32,
    pos: IVec2,
    rot: Rotation,
    dims: UVec2,
    variant: InstantiatedPartVariant,
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

        let variant = match proto {
            PartPrototype::Cargo(c) => InstantiatedPartVariant::Cargo(c, CargoInstanceData::new()),
            PartPrototype::Generic(g) => InstantiatedPartVariant::Generic(g),
            PartPrototype::Machine(m) => {
                InstantiatedPartVariant::Machine(m, MachineInstanceData::default())
            }
            PartPrototype::Magnetorquer(m) => {
                InstantiatedPartVariant::Magnetorquer(m, MagnetorquerInstanceData::new())
            }
            PartPrototype::Radar(r) => InstantiatedPartVariant::Radar(r),
            PartPrototype::Tank(t) => InstantiatedPartVariant::Tank(t, TankInstanceData::default()),
            PartPrototype::Thruster(t) => {
                InstantiatedPartVariant::Thruster(t, ThrusterInstanceData::new())
            }
        };

        Self {
            builds_performed: 0,
            builds_required: (dims.x * dims.y).clamp(30, 2000),
            pos,
            rot,
            dims,
            variant,
        }
    }

    pub fn prototype(&self) -> PartPrototype {
        match self.variant.clone() {
            InstantiatedPartVariant::Thruster(t, _) => PartPrototype::Thruster(t),
            InstantiatedPartVariant::Tank(t, _) => PartPrototype::Tank(t),
            InstantiatedPartVariant::Radar(r) => PartPrototype::Radar(r),
            InstantiatedPartVariant::Cargo(c, _) => PartPrototype::Cargo(c),
            InstantiatedPartVariant::Magnetorquer(m, _) => PartPrototype::Magnetorquer(m),
            InstantiatedPartVariant::Machine(m, _) => PartPrototype::Machine(m),
            InstantiatedPartVariant::Generic(g) => PartPrototype::Generic(g),
        }
    }

    pub fn variant(&self) -> &InstantiatedPartVariant {
        &self.variant
    }

    pub fn total_mass(&self) -> Mass {
        match &self.variant {
            InstantiatedPartVariant::Thruster(t, _) => t.mass(),
            InstantiatedPartVariant::Tank(t, d) => t.dry_mass() + d.contents_mass(),
            InstantiatedPartVariant::Radar(r) => r.mass(),
            InstantiatedPartVariant::Cargo(c, d) => c.empty_mass() + d.contents_mass(),
            InstantiatedPartVariant::Magnetorquer(m, _) => m.mass(),
            InstantiatedPartVariant::Machine(m, _) => m.mass(),
            InstantiatedPartVariant::Generic(g) => g.mass(),
        }
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
            self.origin().as_vec2() / crate::prelude::PIXELS_PER_METER + dims / 2.0,
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

    pub fn as_tank(&self) -> Option<(&TankModel, &TankInstanceData)> {
        if let InstantiatedPartVariant::Tank(t, d) = &self.variant {
            Some((t, d))
        } else {
            None
        }
    }

    pub fn as_tank_mut(&mut self) -> Option<(&TankModel, &mut TankInstanceData)> {
        if let InstantiatedPartVariant::Tank(t, d) = &mut self.variant {
            Some((t, d))
        } else {
            None
        }
    }

    pub fn as_thruster(&self) -> Option<(&ThrusterModel, &ThrusterInstanceData)> {
        if let InstantiatedPartVariant::Thruster(t, d) = &self.variant {
            Some((t, d))
        } else {
            None
        }
    }

    pub fn as_thruster_mut(&mut self) -> Option<(&ThrusterModel, &mut ThrusterInstanceData)> {
        if let InstantiatedPartVariant::Thruster(t, d) = &mut self.variant {
            Some((t, d))
        } else {
            None
        }
    }

    pub fn as_machine(&self) -> Option<(&Machine, &MachineInstanceData)> {
        if let InstantiatedPartVariant::Machine(m, d) = &self.variant {
            Some((m, d))
        } else {
            None
        }
    }

    pub fn as_machine_mut(&mut self) -> Option<(&Machine, &mut MachineInstanceData)> {
        if let InstantiatedPartVariant::Machine(m, d) = &mut self.variant {
            Some((m, d))
        } else {
            None
        }
    }

    pub fn as_cargo(&self) -> Option<(&Cargo, &CargoInstanceData)> {
        if let InstantiatedPartVariant::Cargo(c, d) = &self.variant {
            Some((c, d))
        } else {
            None
        }
    }

    pub fn as_cargo_mut(&mut self) -> Option<(&Cargo, &mut CargoInstanceData)> {
        if let InstantiatedPartVariant::Cargo(c, d) = &mut self.variant {
            Some((c, d))
        } else {
            None
        }
    }

    pub fn as_magnetorquer(&self) -> Option<(&Magnetorquer, &MagnetorquerInstanceData)> {
        if let InstantiatedPartVariant::Magnetorquer(m, d) = &self.variant {
            Some((m, d))
        } else {
            None
        }
    }

    pub fn as_magnetorquer_mut(
        &mut self,
    ) -> Option<(&Magnetorquer, &mut MagnetorquerInstanceData)> {
        if let InstantiatedPartVariant::Magnetorquer(m, d) = &mut self.variant {
            Some((m, d))
        } else {
            None
        }
    }

    pub fn as_radar(&self) -> Option<&Radar> {
        if let InstantiatedPartVariant::Radar(r) = &self.variant {
            Some(r)
        } else {
            None
        }
    }
}
