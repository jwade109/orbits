use crate::aabb::AABB;
use crate::factory::*;
use crate::id::EntityId;
use crate::math::*;
use crate::nanotime::Nanotime;
use crate::parts::*;
use crate::pid::PDCtrl;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

fn rocket_equation(ve: f64, m0: Mass, m1: Mass) -> f64 {
    ve * (m0.to_kg_f64() / m1.to_kg_f64()).ln()
}

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

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct VehiclePd {
    pub attitude_controller: PDCtrl,
    pub vertical_controller: PDCtrl,
    pub horizontal_controller: PDCtrl,
    pub docking_linear_controller: PDCtrl,
}

#[derive(Debug, Clone)]
pub struct Vehicle {
    name: String,
    model: String,
    next_part_id: PartId,
    parts: HashMap<PartId, InstantiatedPart>,
    is_thrust_idle: bool,
    discriminator: u64,

    pub pid: VehiclePd,

    pub gyro: Gyro,

    center_of_mass: DVec2,
    total_mass: Mass,
    moment_of_inertia: f64,
    is_thrusting: bool,
}

impl Vehicle {
    pub fn new() -> Self {
        Self::from_parts(
            "Unnamed Ship".into(),
            "XYZ".into(),
            Vec::new(),
            VehiclePd::default(),
        )
    }

    pub fn from_parts(
        name: String,
        model: String,
        prototypes: Vec<(IVec2, Rotation, PartPrototype)>,
        tuning: VehiclePd,
    ) -> Self {
        let mut next_part_id = PartId(0);
        let mut parts = HashMap::new();

        for (pos, rot, proto) in prototypes {
            let instance = InstantiatedPart::from_prototype(proto, pos, rot);
            parts.insert(next_part_id, instance);

            next_part_id.0 += 1;
        }

        let mut ret = Self {
            name,
            model,
            next_part_id,
            parts,
            is_thrust_idle: false,
            discriminator: 0,

            pid: tuning,

            gyro: Gyro::new(),

            center_of_mass: DVec2::ZERO,
            total_mass: Mass::ZERO,
            moment_of_inertia: 0.0,
            is_thrusting: false,
        };

        ret.update();

        ret
    }

    pub fn discriminator(&self) -> u64 {
        self.discriminator
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
        self.update();
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
        self.update();
        part
    }

    pub fn clear(&mut self) {
        self.parts.clear();
        self.update();
    }

    fn update_discriminator(&mut self) {
        let mut hash = std::hash::DefaultHasher::new();
        if self.parts.is_empty() {
            self.discriminator = 0;
            return;
        }

        let mut hash_stuff = Vec::new();

        for (_, part) in &self.parts {
            let stuff = (
                part.origin(),
                part.rotation(),
                part.prototype().part_name().to_string(),
            );
            hash_stuff.push(stuff);
        }

        hash_stuff.sort_by(|(pa, ra, na), (pb, rb, nb)| {
            use std::cmp::Ordering;
            match pa.x.cmp(&pb.x) {
                Ordering::Less => return Ordering::Less,
                Ordering::Equal => (),
                Ordering::Greater => return Ordering::Greater,
            };

            match pa.y.cmp(&pb.y) {
                Ordering::Less => return Ordering::Less,
                Ordering::Equal => (),
                Ordering::Greater => return Ordering::Greater,
            };

            match ra.cmp(&rb) {
                Ordering::Less => return Ordering::Less,
                Ordering::Equal => (),
                Ordering::Greater => return Ordering::Greater,
            };

            na.cmp(&nb)
        });

        for elem in hash_stuff {
            elem.hash(&mut hash);
        }

        self.discriminator = hash.finish();
    }

    fn update_physical_quantities(&mut self) {
        self.total_mass = if self.parts.is_empty() {
            Mass::kilograms(100)
        } else {
            self.parts.iter().map(|(_, p)| p.total_mass()).sum()
        };

        self.moment_of_inertia = if self.parts.is_empty() {
            1000.0
        } else {
            let com = self.center_of_mass();
            let mut moa = 0.0;
            for (_, part) in &self.parts {
                let mass = part.total_mass();
                let center = part.center_meters().as_dvec2();
                let rsq = center.distance_squared(com);
                moa += rsq * mass.to_kg_f64()
            }
            moa
        };

        self.center_of_mass = self
            .parts
            .iter()
            .map(|(_, p)| {
                let center = p.origin().as_vec2() / PIXELS_PER_METER + p.dims_meters() / 2.0;
                let weight = p.total_mass().to_kg_f64() / self.total_mass.to_kg_f64();
                center.as_dvec2() * weight
            })
            .sum();
    }

    fn update(&mut self) {
        self.update_discriminator();
        self.update_physical_quantities();
    }

    pub fn parts(&self) -> impl Iterator<Item = (&PartId, &InstantiatedPart)> + use<'_> {
        self.parts.iter()
    }

    pub fn parts_in_draw_order(
        &self,
    ) -> impl Iterator<Item = (&PartId, &InstantiatedPart)> + use<'_> {
        PartLayer::draw_order()
            .map(|l| self.parts.iter().filter(move |(_, p)| p.layer() == l))
            .into_iter()
            .flat_map(|p| p.into_iter())
    }

    pub fn fuel_percentage(&self) -> f64 {
        0.0
    }

    pub fn is_controllable(&self) -> bool {
        false
    }

    pub fn dry_mass(&self) -> Mass {
        self.total_mass() - self.fuel_mass()
    }

    pub fn fuel_mass(&self) -> Mass {
        Mass::ZERO
    }

    pub fn total_mass(&self) -> Mass {
        self.total_mass
    }

    pub fn thruster_count(&self) -> usize {
        self.thrusters().count()
    }

    pub fn tank_count(&self) -> usize {
        0
    }

    pub fn max_forward_thrust(&self) -> f64 {
        0.0
    }

    pub fn max_backwards_thrust(&self) -> f64 {
        0.0
    }

    pub fn center_of_mass(&self) -> DVec2 {
        self.center_of_mass
    }

    pub fn moment_of_inertia(&self) -> f64 {
        self.moment_of_inertia
    }

    pub fn aabb(&self) -> AABB {
        let mut ret: Option<AABB> = None;
        for (_, instance) in &self.parts {
            let dims = instance.dims_meters();
            let pos = instance.origin().as_vec2() / crate::parts::parts::PIXELS_PER_METER;
            let aabb = AABB::from_arbitrary(pos, pos + dims);
            if let Some(r) = ret.as_mut() {
                r.include(&pos);
                r.include(&(pos + dims));
            } else {
                ret = Some(aabb);
            }
        }
        ret.unwrap_or(AABB::unit())
    }

    pub fn pixel_bounds(&self) -> Option<(IVec2, IVec2)> {
        let mut min: Option<IVec2> = None;
        let mut max: Option<IVec2> = None;
        for (_, instance) in &self.parts {
            let dims = instance.dims_grid();
            let origin = instance.origin();
            let upper = origin + dims.as_ivec2();
            if let Some((min, max)) = min.as_mut().zip(max.as_mut()) {
                min.x = min.x.min(origin.x);
                min.y = min.y.min(origin.y);
                max.x = max.x.max(upper.x);
                max.y = max.y.max(upper.y);
            } else {
                min = Some(origin);
                max = Some(upper);
            }
        }
        min.zip(max)
    }

    pub fn low_fuel(&self) -> bool {
        false
        // self.is_controllable() && self.remaining_dv() < 50.0
    }

    pub fn is_thrusting(&self) -> bool {
        self.is_thrusting
        // self.thrusters().any(|(t, d)| d.is_thrusting(t))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn name_with_id(&self, id: EntityId) -> String {
        format!("{} {}", self.name, id)
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }

    pub fn title(&self) -> String {
        let model = if self.model.len() >= 3 {
            self.model[0..3].to_uppercase()
        } else {
            self.model.to_uppercase()
        };
        format!("{} {}", model, self.name)
    }

    pub fn title_with_id(&self, id: EntityId) -> String {
        let title = self.title();
        format!("{} {}", title, id)
    }

    fn current_angular_acceleration(&self) -> f64 {
        0.0
    }

    pub fn thrusters(&self) -> impl Iterator<Item = &ThrusterModel> + use<'_> {
        self.parts.iter().filter_map(|(_, p)| p.thruster_data())
    }

    pub fn bounding_radius(&self) -> f64 {
        let aabb = self.aabb();
        let mut r: f64 = 0.0;
        for c in aabb.corners() {
            r = r.max(c.length() as f64);
        }
        r
    }

    pub fn build_part(&mut self, id: PartId) {
        if let Some(part) = self.parts.get_mut(&id) {
            part.build();
        }
    }

    pub fn build_all(&mut self) {
        for (_, part) in &mut self.parts {
            part.build_all();
        }
    }

    pub fn build_once(&mut self) {
        for layer in PartLayer::build_order() {
            let layer_is_built = self
                .parts
                .iter()
                .filter(|(_, p)| p.prototype().layer() == layer)
                .all(|(_, p)| p.percent_built() == 1.0);

            if layer_is_built {
                continue;
            }

            for (_, instance) in &mut self.parts {
                if instance.prototype().layer() != layer {
                    continue;
                }

                if instance.percent_built() < 1.0 {
                    if rand(0.0, 1.0) < 0.8 {
                        instance.build();
                    }
                }
            }
            return;
        }
    }

    pub fn normalize_coordinates(&mut self) {
        if self.parts.len() == 0 {
            return;
        }

        let mut min: IVec2 = IVec2::ZERO;
        let mut max: IVec2 = IVec2::ZERO;

        self.parts.iter().for_each(|(_, instance)| {
            let dims = instance.dims_grid();
            let p = instance.origin();
            let q = p + dims.as_ivec2();
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            max.x = max.x.max(q.x);
            max.y = max.y.max(q.y);
        });

        let avg = min + (max - min) / 2;

        self.parts.iter_mut().for_each(|(_, p)| {
            p.set_origin(p.origin() - avg);
        });

        self.update();
    }
}
