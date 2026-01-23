use crate::prelude::*;
use bevy::prelude::Component;
use std::collections::BTreeMap;
use std::hash::Hash;

#[allow(unused)]
fn mass_after_maneuver(ve: f64, m0: f64, dv: f64) -> f64 {
    m0 / (dv / ve).exp()
}

pub const PHYSICS_CONSTANT_UPDATE_RATE: u32 = 40;

pub const PHYSICS_CONSTANT_DELTA_TIME: Nanotime =
    Nanotime::millis(1000 / PHYSICS_CONSTANT_UPDATE_RATE as i64);

pub fn occupied_cells(pos: PartCoord, rot: Rotation, part: &PartPrototype) -> Vec<PartCoord> {
    let mut ret = vec![];
    let wh = pixel_dims_with_rotation(rot, part);
    for w in 0..wh.x {
        for h in 0..wh.y {
            let p = pos + PartCoord::new(UVec2::new(w, h).as_ivec2());
            ret.push(p);
        }
    }
    ret
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PartId(u64);

#[derive(Debug, Clone, Component)]
pub struct Blueprint {
    next_part_id: PartId,
    parts: BTreeMap<PartId, InstantiatedPart>,
    pipes: BTreeMap<PartId, PipeGeometry>,
}

impl Blueprint {
    pub fn new() -> Self {
        Self {
            next_part_id: PartId(0),
            parts: BTreeMap::new(),
            pipes: BTreeMap::new(),
        }
    }

    pub fn merge(&mut self, other: &Blueprint) {
        for (_, part) in &other.parts {
            self.add_part(part.proto.clone(), part.pos, part.rot);
        }
        for (_, pipe) in &other.pipes {
            self.add_pipe(*pipe);
        }
    }

    pub fn from_parts(
        prototypes: Vec<(PartCoord, Rotation, PartPrototype)>,
        pipes: Vec<PipeGeometry>,
    ) -> Self {
        let mut next_part_id = PartId(0);
        let mut parts = BTreeMap::new();

        for (pos, rot, proto) in prototypes {
            let instance = InstantiatedPart::from_prototype(proto, pos, rot);
            parts.insert(next_part_id, instance);

            next_part_id.0 += 1;
        }

        let mut s = Self {
            next_part_id,
            parts,
            pipes: BTreeMap::new(),
        };

        for pipe in pipes {
            s.add_pipe(pipe);
        }

        s
    }

    fn get_next_part_id(&mut self) -> PartId {
        let ret = self.next_part_id;
        self.next_part_id.0 += 1;
        ret
    }

    pub fn add_part(&mut self, proto: PartPrototype, pos: PartCoord, rot: Rotation) -> PartId {
        let id = self.get_next_part_id();
        let instance = InstantiatedPart::from_prototype(proto, pos, rot);
        self.parts.insert(id, instance);
        id
    }

    pub fn get_part(&self, id: PartId) -> Option<&InstantiatedPart> {
        self.parts.get(&id)
    }

    pub fn get_pipe(&self, id: PartId) -> Option<&PipeGeometry> {
        self.pipes.get(&id)
    }

    pub fn get_part_at_layer(&self, p: PartCoord, layer: PartLayer) -> Option<PartId> {
        // TODO make this spatial lookup faster

        if layer == PartLayer::Plumbing {
            let found = self.pipes.iter().find(|(_, pipe)| pipe.contains(p));
            return found.map(|(id, _)| *id);
        }

        let found = self.parts.iter().find(|(_, instance)| {
            if layer != instance.prototype().layer() {
                return false;
            }

            let origin = instance.origin();
            let dims = instance.dims_grid().as_ivec2();
            let p = p - origin;
            let inner = p.inner();
            inner.x >= 0 && inner.y >= 0 && inner.x < dims.x && inner.y < dims.y
        });

        found.map(|(id, _)| *id)
    }

    pub fn get_part_at(&self, p: PartCoord, layer: impl Into<Option<PartLayer>>) -> Option<PartId> {
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
                let inner = p.inner();
                inner.x >= 0 && inner.y >= 0 && inner.x < dims.x && inner.y < dims.y
            });

            if let Some((id, _)) = found {
                return Some(*id);
            }
        }

        None
    }

    pub fn remove_part_at(&mut self, p: PartCoord, layer: impl Into<Option<PartLayer>>) -> bool {
        let layer = layer.into();

        // try to remove a pipe first
        for id in self.get_pipes_at(p) {
            if self.remove_part(id) {
                return true;
            }
        }

        if let Some(layer) = layer {
            if let Some(id) = self.get_part_at(p, layer) {
                self.remove_part(id)
            } else {
                false
            }
        } else {
            let mut layers = PartLayer::draw_order();
            layers.reverse();
            for layer in layers {
                if self.remove_part_at(p, layer) {
                    return true;
                }
            }
            false
        }
    }

    pub fn remove_part(&mut self, id: PartId) -> bool {
        if self.parts.remove(&id).is_some() {
            return true;
        }
        if self.pipes.remove(&id).is_some() {
            return true;
        }
        false
    }

    pub fn clear(&mut self) {
        self.parts.clear();
        self.pipes.clear();
    }

    pub fn parts(&self) -> impl Iterator<Item = (&PartId, &InstantiatedPart)> + use<'_> {
        self.parts.iter()
    }

    pub fn pipes(&self) -> impl Iterator<Item = (&PartId, &PipeGeometry)> + use<'_> {
        self.pipes.iter()
    }

    pub fn bounding_radius(&self) -> f64 {
        // BIG TODO
        50.0
    }

    pub fn add_pipe(&mut self, pipe: PipeGeometry) -> PartId {
        let id = self.get_next_part_id();
        self.pipes.insert(id, pipe);
        id
    }

    pub fn get_pipes_at(&self, p: PartCoord) -> Vec<PartId> {
        self.pipes
            .iter()
            .filter_map(|(id, pipe)| pipe.contains(p).then(|| *id))
            .collect()
    }

    pub fn rotate_ccw(&mut self) {
        for (_, part) in &mut self.parts {
            part.rotate_ccw();
        }

        for (_, pipe) in &mut self.pipes {
            pipe.rotate_ccw();
        }
    }

    pub fn bounds(&self) -> (PartCoord, PartCoord) {
        if self.parts.is_empty() {
            return (IVec2::ZERO.into(), IVec2::ZERO.into());
        }

        let mut lower: Option<IVec2> = None;
        let mut upper: Option<IVec2> = None;

        for (_, part) in &self.parts {
            let l = part.pos.inner();
            let u = l + part.dims_grid().as_ivec2();

            lower = Some(
                lower
                    .map(|k| IVec2::new(k.x.min(l.x), k.y.min(l.y)))
                    .unwrap_or(l),
            );
            upper = Some(
                upper
                    .map(|k| IVec2::new(k.x.max(u.x), k.y.max(u.y)))
                    .unwrap_or(l),
            );
        }

        (lower.unwrap().into(), upper.unwrap().into())
    }

    pub fn shift(&mut self, delta: IVec2) {
        self.parts.iter_mut().for_each(|(_, p)| {
            p.pos = p.pos + PartCoord::new(delta);
        });

        self.pipes.iter_mut().for_each(|(_, p)| {
            p.start = p.start + PartCoord::new(delta);
            p.end = p.end + PartCoord::new(delta);
        });
    }

    pub fn center(&self) -> PartCoord {
        let (lower, upper) = self.bounds();
        PartCoord::new((upper.inner() + lower.inner()) / 2)
    }

    pub fn dims(&self) -> UVec2 {
        let (lower, upper) = self.bounds();
        (upper - lower).inner().as_uvec2()
    }

    pub fn normalize_coordinates(&mut self) {
        if self.parts.is_empty() {
            return;
        }
        let (lower, upper) = self.bounds();
        let center = (upper.inner() + lower.inner()) / 2;
        self.shift(-center);
    }

    // for each pipe, returns that pipe's ID, and the ID of the part
    // each end is connected to
    pub fn pipe_connections(&self) -> Vec<(PartId, PartId, PartId)> {
        let mut ret = Vec::new();
        for (id, pipe) in &self.pipes {
            let Some(s) = self.get_part_at(pipe.start, PartLayer::Internal) else {
                continue;
            };
            let Some(e) = self.get_part_at(pipe.end, PartLayer::Internal) else {
                continue;
            };
            ret.push((*id, s, e));
        }
        ret
    }
}
