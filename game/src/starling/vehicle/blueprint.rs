use crate::starling::math::*;
use crate::starling::nanotime::Nanotime;
use crate::starling::parts::*;
use crate::starling::pid::PDCtrl;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

#[allow(unused)]
fn mass_after_maneuver(ve: f64, m0: f64, dv: f64) -> f64 {
    m0 / (dv / ve).exp()
}

pub const PHYSICS_CONSTANT_UPDATE_RATE: u32 = 40;

pub const PHYSICS_CONSTANT_DELTA_TIME: Nanotime =
    Nanotime::millis(1000 / PHYSICS_CONSTANT_UPDATE_RATE as i64);

pub fn occupied_pixels(pos: IVec2, rot: Rotation, part: &PartPrototype) -> Vec<IVec2> {
    let mut ret = vec![];
    let wh = pixel_dims_with_rotation(rot, part);
    for w in 0..wh.x {
        for h in 0..wh.y {
            let p = pos + UVec2::new(w, h).as_ivec2();
            ret.push(p);
        }
    }
    ret
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartId(u64);

#[derive(Debug, Clone)]
pub struct Blueprint {
    next_part_id: PartId,
    parts: HashMap<PartId, InstantiatedPart>,
}

impl Blueprint {
    pub fn new() -> Self {
        Self {
            next_part_id: PartId(0),
            parts: HashMap::new(),
        }
    }

    pub fn from_parts(prototypes: Vec<(IVec2, Rotation, PartPrototype)>) -> Self {
        let mut next_part_id = PartId(0);
        let mut parts = HashMap::new();

        for (pos, rot, proto) in prototypes {
            let instance = InstantiatedPart::from_prototype(proto, pos, rot);
            parts.insert(next_part_id, instance);

            next_part_id.0 += 1;
        }

        Self {
            next_part_id,
            parts,
        }
    }

    fn get_next_part_id(&mut self) -> PartId {
        let ret = self.next_part_id;
        self.next_part_id.0 += 1;
        ret
    }

    pub fn add_part(&mut self, proto: PartPrototype, pos: IVec2, rot: Rotation) -> PartId {
        let id = self.get_next_part_id();
        let instance = InstantiatedPart::from_prototype(proto, pos, rot);
        self.parts.insert(id, instance);
        id
    }

    pub fn get_part(&self, id: PartId) -> Option<&InstantiatedPart> {
        self.parts.get(&id)
    }

    pub fn get_part_at(&self, p: IVec2, layer: impl Into<Option<PartLayer>>) -> Option<PartId> {
        let layer: Option<PartLayer> = layer.into();

        for part_layer in enum_iterator::reverse_all::<PartLayer>() {
            let found = self.parts.iter().find(|(_, instance)| {
                if let Some(layer) = layer {
                    if layer != instance.prototype().layer() {
                        return false;
                    }
                }

                if instance.prototype().layer() != part_layer {
                    return false;
                }

                let origin = instance.origin();
                let dims = instance.dims_grid().as_ivec2();
                let p = p - origin;
                p.x >= 0 && p.y >= 0 && p.x <= dims.x && p.y <= dims.y
            });

            if let Some((id, _)) = found {
                return Some(*id);
            }
        }

        None
    }

    pub fn remove_part_at(
        &mut self,
        p: IVec2,
        layer: impl Into<Option<PartLayer>>,
    ) -> Result<InstantiatedPart, &'static str> {
        let layer = layer.into();

        if let Some(layer) = layer {
            let id = self
                .get_part_at(p, layer)
                .ok_or("No part at given position and layer")?;
            self.remove_part(id).ok_or("No part with given ID")
        } else {
            let mut layers = PartLayer::draw_order();
            layers.reverse();
            for layer in layers {
                if let Some(part) = self
                    .get_part_at(p, layer)
                    .map(|id| self.remove_part(id))
                    .flatten()
                {
                    return Ok(part);
                }
            }
            Err("No part found")
        }
    }

    pub fn remove_part(&mut self, id: PartId) -> Option<InstantiatedPart> {
        let part = self.parts.remove(&id);
        part
    }

    pub fn clear(&mut self) {
        self.parts.clear();
    }

    pub fn parts(&self) -> impl Iterator<Item = (&PartId, &InstantiatedPart)> + use<'_> {
        self.parts.iter()
    }

    pub fn bounding_radius(&self) -> f64 {
        // BIG TODO
        50.0
    }
}
