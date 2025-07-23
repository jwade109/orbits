use crate::aabb::AABB;
use crate::factory::*;
use crate::math::*;
use crate::nanotime::Nanotime;
use crate::parts::*;
use crate::vehicle::*;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

fn rocket_equation(ve: f32, m0: Mass, m1: Mass) -> f32 {
    ve * (m0.to_kg_f32() / m1.to_kg_f32()).ln()
}

#[allow(unused)]
fn mass_after_maneuver(ve: f32, m0: f32, dv: f32) -> f32 {
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
pub struct Vehicle {
    name: String,
    pipes: HashSet<IVec2>,
    next_part_id: PartId,
    parts: HashMap<PartId, InstantiatedPart>,
    conn_groups: Vec<ConnectivityGroup>,
    is_thrust_idle: bool,
    discriminator: u64,
}

impl Vehicle {
    pub fn new() -> Self {
        Self::from_parts("".into(), Vec::new(), HashSet::new())
    }

    pub fn from_parts(
        name: String,
        prototypes: Vec<(IVec2, Rotation, PartPrototype)>,
        pipes: HashSet<IVec2>,
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
            next_part_id,
            parts,
            pipes,
            conn_groups: Vec::new(),
            is_thrust_idle: false,
            discriminator: 0,
        };

        ret.update();

        ret
    }

    pub fn discriminator(&self) -> u64 {
        self.discriminator
    }

    pub fn add_pipe(&mut self, p: IVec2) {
        self.pipes.insert(p);
        self.update();
    }

    pub fn remove_pipe(&mut self, p: IVec2) {
        self.pipes.remove(&p);
        self.update();
    }

    pub fn has_pipe(&mut self, p: IVec2) -> bool {
        self.pipes.contains(&p)
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
        self.pipes.clear();
        self.update();
    }

    fn construct_connectivity(&mut self) {
        // visit all pipe locations
        let mut all_pipes = self.pipes.clone();
        let mut open_set = HashSet::new();

        let mut conn_groups = Vec::new();

        while let Some(p) = all_pipes.iter().next() {
            open_set.insert(*p);

            let mut local_graph = ConnectivityGroup::new();

            while let Some(p) = open_set.iter().next().cloned() {
                open_set.remove(&p);
                if !all_pipes.contains(&p) {
                    continue;
                }
                all_pipes.remove(&p);

                local_graph.add_transport_line(p);

                if let Some(id) = self.get_part_at(p, PartLayer::Internal) {
                    if let Some(q) = local_graph.get_pos(id) {
                        let center = if let Some(part) = self.get_part(id) {
                            part.origin() + part.dims_grid().as_ivec2() / 2
                        } else {
                            IVec2::ZERO
                        };
                        if p.distance_squared(center) < q.distance_squared(center) {
                            local_graph.connect(id, p);
                        }
                    } else {
                        local_graph.connect(id, p);
                    }
                }

                for off in [IVec2::X, IVec2::Y, -IVec2::X, -IVec2::Y] {
                    let neighbor = p - off;
                    if self.pipes.contains(&neighbor) {
                        open_set.insert(neighbor);
                    }
                }
            }

            if local_graph.len() > 1 {
                conn_groups.push(local_graph);
            }
        }

        self.conn_groups = conn_groups;
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

    pub fn conn_groups(&self) -> impl Iterator<Item = &ConnectivityGroup> + use<'_> {
        self.conn_groups.iter()
    }

    pub fn is_connected(&self, id_a: PartId, id_b: PartId) -> bool {
        self.conn_groups.iter().any(|g| g.is_connected(id_a, id_b))
    }

    fn update(&mut self) {
        self.construct_connectivity();
        self.update_discriminator();
    }

    pub fn pipes(&self) -> impl Iterator<Item = IVec2> + use<'_> {
        self.pipes.iter().cloned()
    }

    pub fn parts(&self) -> impl Iterator<Item = (&PartId, &InstantiatedPart)> + use<'_> {
        self.parts.iter()
    }

    pub fn fuel_percentage(&self) -> f32 {
        let max_fuel_mass: Mass = self.tanks().map(|(t, _)| t.max_fluid_mass).sum();
        if max_fuel_mass == Mass::ZERO {
            return 0.0;
        }
        let current_fuel_mass: Mass = self.tanks().map(|(_, d)| d.contents_mass()).sum();
        current_fuel_mass.to_kg_f32() / max_fuel_mass.to_kg_f32()
    }

    pub fn is_controllable(&self) -> bool {
        // TODO
        true
    }

    pub fn dry_mass(&self) -> Mass {
        self.total_mass() - self.fuel_mass()
    }

    pub fn fuel_mass(&self) -> Mass {
        if self.parts.is_empty() {
            return Mass::ZERO;
        }
        self.tanks().map(|(_, d)| d.contents_mass()).sum()
    }

    pub fn total_mass(&self) -> Mass {
        if self.parts.is_empty() {
            return Mass::kilograms(100);
        }
        self.parts.iter().map(|(_, p)| p.total_mass()).sum()
    }

    pub fn thruster_count(&self) -> usize {
        self.thrusters().count()
    }

    pub fn tank_count(&self) -> usize {
        self.tanks().count()
    }

    pub fn max_thrust(&self) -> f32 {
        if self.thruster_count() == 0 {
            0.0
        } else {
            self.thrusters().map(|(t, _)| t.max_thrust()).sum()
        }
    }

    fn thrust_along_heading(&self, angle: f32, rcs: bool, current: bool) -> f32 {
        if self.thruster_count() == 0 {
            return 0.0;
        }

        let u = rotate(Vec2::X, angle);

        let mut sum = 0.0;

        for (_, part) in &self.parts {
            if let Some((t, d)) = part.as_thruster() {
                if t.is_rcs != rcs {
                    continue;
                }
                let v = rotate(Vec2::X, part.rotation().to_angle());
                let dot = u.dot(v).max(0.0);
                sum += dot
                    * if current {
                        t.current_thrust(d)
                    } else {
                        t.max_thrust()
                    };
            }
        }

        sum
    }

    pub fn max_thrust_along_heading(&self, angle: f32, rcs: bool) -> f32 {
        self.thrust_along_heading(angle, rcs, false)
    }

    pub fn current_thrust_along_heading(&self, angle: f32, rcs: bool) -> f32 {
        self.thrust_along_heading(angle, rcs, true)
    }

    pub fn center_of_mass(&self) -> Vec2 {
        let mass = self.total_mass();
        self.parts
            .iter()
            .map(|(_, p)| {
                let center = p.origin().as_vec2() / PIXELS_PER_METER + p.dims_meters() / 2.0;
                let weight = p.total_mass().to_kg_f32() / mass.to_kg_f32();
                center * weight
            })
            .sum()
    }

    pub fn moment_of_inertia(&self) -> f32 {
        if self.parts.is_empty() {
            return 1.0;
        }
        let com = self.center_of_mass();
        let mut moa = 0.0;
        for (_, part) in &self.parts {
            let mass = part.total_mass();
            let center = part.center_meters();
            let rsq = center.distance_squared(com);
            moa += rsq * mass.to_kg_f32()
        }
        moa
    }

    pub fn accel(&self) -> f32 {
        let thrust = self.max_thrust();
        let mass = self.total_mass();
        if mass == Mass::ZERO {
            0.0
        } else {
            thrust / mass.to_kg_f32()
        }
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
        self.is_controllable() && self.remaining_dv() < 50.0
    }

    pub fn is_thrusting(&self) -> bool {
        self.thrusters().any(|(t, d)| d.is_thrusting(t))
    }

    pub fn has_radar(&self) -> bool {
        !self.radars().count() > 0
    }

    pub fn average_linear_exhaust_velocity(&self) -> f32 {
        let linear_thrusters: Vec<_> = self.thrusters().filter(|(t, _)| !t.is_rcs()).collect();

        let count = linear_thrusters.len();

        if count == 0 {
            return 0.0;
        }

        linear_thrusters
            .into_iter()
            .map(|(t, _)| t.exhaust_velocity / count as f32)
            .sum()
    }

    pub fn fuel_consumption_rate(&self) -> f32 {
        self.thrusters()
            .map(|(t, d)| t.fuel_consumption_rate(d))
            .sum()
    }

    pub fn remaining_dv(&self) -> f32 {
        if self.total_mass() == Mass::ZERO || self.dry_mass() == Mass::ZERO {
            return 0.0;
        }
        let ve = self.average_linear_exhaust_velocity();
        rocket_equation(ve, self.total_mass(), self.dry_mass())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn current_angular_acceleration(&self) -> f32 {
        let mut aa = 0.0;
        let moa = self.moment_of_inertia();
        let com = self.center_of_mass();

        for (_, part) in &self.parts {
            if let Some((t, d)) = part.as_thruster() {
                let center_of_thrust = part.center_meters();
                let lever_arm = center_of_thrust - com;
                let thrust_dir = rotate(Vec2::X, part.rotation().to_angle());
                let torque = cross2d(lever_arm, thrust_dir) * t.current_thrust(d);
                aa += torque / moa;
            }

            if let Some((_, d)) = part.as_magnetorquer() {
                aa += d.torque() / moa;
            }
        }

        aa
    }

    fn current_body_frame_linear_acceleration(&self) -> Vec2 {
        let mut body_frame_force = Vec2::ZERO;
        let mass = self.total_mass().to_kg_f32();

        for (_, part) in &self.parts {
            if let Some((t, d)) = part.as_thruster() {
                let thrust_dir = rotate(Vec2::X, part.rotation().to_angle());
                body_frame_force += thrust_dir * t.current_thrust(d);
            }
        }

        body_frame_force / mass
    }

    pub fn set_thrust_control(&mut self, control: VehicleControl) {
        let is_nullopt = control == VehicleControl::NULLOPT;

        if self.is_thrust_idle && is_nullopt {
            // nothing to do
            return;
        }

        let com = self.center_of_mass();

        for (_, part) in &mut self.parts {
            let rot = part.rotation();
            let center_of_thrust = part.center_meters();
            let u = rotate(Vec2::X, part.rotation().to_angle());
            if let Some((t, d)) = part.as_thruster_mut() {
                let linear_command = match rot {
                    Rotation::East => control.plus_x,
                    Rotation::North => control.plus_y,
                    Rotation::West => control.neg_x,
                    Rotation::South => control.neg_y,
                };

                let throttle = if t.is_rcs {
                    // this is an RCS thruster

                    let linear_throttle = if linear_command.use_rcs {
                        // we're using RCS for linear translation
                        linear_command.throttle
                    } else {
                        0.0
                    };

                    // also fire this thruster if it turns the vehicle
                    // the right way
                    let is_torque = {
                        let torque = cross2d(center_of_thrust - com, u);
                        torque.signum() == control.attitude.signum()
                    };
                    linear_throttle
                        + if is_torque {
                            control.attitude.abs()
                        } else {
                            0.0
                        }
                } else {
                    if !linear_command.use_rcs {
                        linear_command.throttle
                    } else {
                        0.0
                    }
                };

                d.set_throttle(throttle);
            }
        }

        self.is_thrust_idle = is_nullopt;
    }

    pub fn on_sim_tick(&mut self) {
        let mut machines = Vec::new();

        for (id, part) in &mut self.parts {
            if part.percent_built() < 1.0 {
                continue;
            }

            if let Some((t, d)) = part.as_thruster_mut() {
                d.on_sim_tick(t);
            }

            if let Some((_, d)) = part.as_machine_mut() {
                d.on_sim_tick();
                machines.push(id);
            }
        }

        let mut tank_ids = HashSet::new();

        for id in machines {
            for conn in &self.conn_groups {
                if !conn.contains(*id) {
                    continue;
                }
                for other in conn.ids() {
                    if other == *id {
                        continue;
                    }
                    tank_ids.insert(other);
                }
            }
        }

        for id in tank_ids {
            if let Some(p) = self.parts.get_mut(&id) {
                if let Some((t, d)) = p.as_tank_mut() {
                    t.put(Item::H2, Mass::kilograms(3), d);
                }
            }
        }
    }

    pub fn body_frame_accel(&self) -> BodyFrameAccel {
        let linear = self.current_body_frame_linear_acceleration();
        let angular = self.current_angular_acceleration();
        BodyFrameAccel { linear, angular }
    }

    pub fn set_all_thrusters(&mut self, throttle: f32) {
        for (_, part) in &mut self.parts {
            if let Some((_, d)) = part.as_thruster_mut() {
                d.set_throttle(throttle);
            }
        }
    }

    pub fn radars(&self) -> impl Iterator<Item = &Radar> + use<'_> {
        self.parts.iter().filter_map(|(_, p)| p.as_radar())
    }

    pub fn magnetorquers(
        &self,
    ) -> impl Iterator<Item = (&Magnetorquer, &MagnetorquerInstanceData)> + use<'_> {
        self.parts.iter().filter_map(|(_, p)| p.as_magnetorquer())
    }

    pub fn tanks(&self) -> impl Iterator<Item = (&TankModel, &TankInstanceData)> + use<'_> {
        self.parts.iter().filter_map(|(_, p)| p.as_tank())
    }

    pub fn thrusters(
        &self,
    ) -> impl Iterator<Item = (&ThrusterModel, &ThrusterInstanceData)> + use<'_> {
        self.parts.iter().filter_map(|(_, p)| p.as_thruster())
    }

    pub fn set_recipe(&mut self, id: PartId, recipe: RecipeListing) -> bool {
        if let Some(part) = self.parts.get_mut(&id) {
            if let Some((_, d)) = part.as_machine_mut() {
                d.recipe = recipe;
                return true;
            }
        }
        false
    }

    pub fn clear_contents(&mut self, id: PartId) -> bool {
        if let Some(part) = self.parts.get_mut(&id) {
            if let Some((_, d)) = part.as_tank_mut() {
                d.clear_contents();
                return true;
            }

            if let Some((_, d)) = part.as_cargo_mut() {
                d.clear_contents();
                return true;
            }
        }

        return false;
    }

    pub fn bounding_radius(&self) -> f32 {
        let aabb = self.aabb();
        let mut r: f32 = 0.0;
        for c in aabb.corners() {
            r = r.max(c.length());
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

        let new_pipes = self.pipes.iter().map(|p| p - avg).collect();
        self.pipes = new_pipes;

        self.update();
    }
}
