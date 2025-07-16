use crate::aabb::AABB;
use crate::factory::{Item, Mass};
use crate::math::*;
use crate::nanotime::Nanotime;
use crate::orbits::{wrap_0_2pi, wrap_pi_npi};
use crate::parts::*;
use crate::pid::*;
use crate::pv::PV;
use crate::vehicle::*;
use std::collections::{HashMap, HashSet};

fn rocket_equation(ve: f32, m0: Mass, m1: Mass) -> f32 {
    ve * (m0.to_kg_f32() / m1.to_kg_f32()).ln()
}

#[allow(unused)]
fn mass_after_maneuver(ve: f32, m0: f32, dv: f32) -> f32 {
    m0 / (dv / ve).exp()
}

#[derive(Default, Debug, Clone, Copy)]
pub struct VehicleControl {
    pub throttle: f32,
    pub linear: Vec2,
    pub attitude: f32,
    pub allow_linear_rcs: bool,
    pub allow_attitude_rcs: bool,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum VehicleControlPolicy {
    #[default]
    Idle,
    External(VehicleControl),
    PositionHold(Vec2),
}

const ATTITUDE_CONTROLLER: PDCtrl = PDCtrl::new(30.0, 35.0);

const VERTICAL_CONTROLLER: PDCtrl = PDCtrl::new(0.2, 1.0);

const HORIZONTAL_CONTROLLER: PDCtrl = PDCtrl::new(0.01, 0.08);

const DOCKING_LINEAR_CONTROLLER: PDCtrl = PDCtrl::new(30.0, 300.0);

fn zero_gravity_control_law(vehicle: &Vehicle) -> VehicleControl {
    let target = if let VehicleControlPolicy::PositionHold(target) = vehicle.policy {
        target
    } else {
        return VehicleControl::default();
    };

    let position_error = target - vehicle.pv.pos_f32();
    let error_dir = position_error.normalize_or_zero();

    let target_angle = Vec2::X.angle_to(position_error);

    let attitude = compute_attitude_control(vehicle, target_angle, &ATTITUDE_CONTROLLER);

    let target_velocity = Vec2::X * 5.0;

    let (linear, throttle) = if attitude > 6.0 {
        (Vec2::ZERO, 0.0)
    } else {
        let px = 0.0;
        let py = 0.0;

        let vx = -target_velocity.x + error_dir.dot(vehicle.pv.vel_f32());
        let vy = -target_velocity.y + rotate(error_dir, PI / 2.0).dot(vehicle.pv.vel_f32());

        let cx = DOCKING_LINEAR_CONTROLLER.apply(px, vx);
        let cy = DOCKING_LINEAR_CONTROLLER.apply(py, vy);

        if cx.abs() < 10.0 && cy.abs() < 10.0 {
            (Vec2::ZERO, 0.0)
        } else if cx.abs() > cy.abs() {
            (Vec2::X * cx, cx.abs())
        } else {
            (Vec2::Y * cy, cy.abs())
        }
    };

    VehicleControl {
        throttle,
        linear,
        attitude,
        allow_linear_rcs: true,
        allow_attitude_rcs: true,
    }
}

fn compute_attitude_control(v: &Vehicle, target_angle: f32, pid: &PDCtrl) -> f32 {
    let attitude_error = wrap_pi_npi(target_angle - v.angle());
    pid.apply(attitude_error, v.angular_velocity())
}

fn hover_control_law(gravity: Vec2, vehicle: &Vehicle) -> VehicleControl {
    let future_alt = vehicle.kinematic_apoapis(gravity.length() as f64) as f32;

    let target = if let VehicleControlPolicy::PositionHold(target) = vehicle.policy {
        target
    } else {
        return VehicleControl::default();
    };

    let target = if target.distance(vehicle.pv.pos_f32()) > 250.0 {
        let d = target - vehicle.pv.pos_f32();
        d.normalize_or_zero() * 250.0 + vehicle.pv.pos_f32()
    } else {
        target
    };

    let horizontal_control =
        HORIZONTAL_CONTROLLER.apply(target.x - vehicle.pv.pos.x as f32, vehicle.pv.vel.x as f32);

    // attitude controller
    let target_angle = PI * 0.5 - horizontal_control.clamp(-PI / 4.0, PI / 4.0);
    let attitude_error = wrap_pi_npi(target_angle - vehicle.angle());
    let attitude = compute_attitude_control(vehicle, target_angle, &ATTITUDE_CONTROLLER);

    let thrust = vehicle.max_thrust_along_heading(0.0, false);
    let accel = thrust / vehicle.current_mass().to_kg_f32();
    let pct = gravity.length() / accel;

    // vertical controller
    let error = VERTICAL_CONTROLLER.apply(target.y - future_alt, vehicle.pv.vel.y as f32);

    let linear = if attitude_error.abs() < 0.5 || vehicle.pv.pos.y > 10.0 {
        Vec2::X
    } else {
        Vec2::ZERO
    };

    let throttle = pct + error;

    VehicleControl {
        throttle,
        linear,
        attitude,
        allow_linear_rcs: false,
        allow_attitude_rcs: true,
    }
}

pub fn current_control_law(vehicle: &Vehicle, gravity: Vec2) -> VehicleControl {
    if let VehicleControlPolicy::External(ctrl) = vehicle.policy {
        return ctrl;
    }

    if gravity.length() > 0.0 {
        hover_control_law(gravity, vehicle)
    } else {
        zero_gravity_control_law(vehicle)
    }
}

pub const PHYSICS_CONSTANT_UPDATE_RATE: u32 = 40;

pub const PHYSICS_CONSTANT_DELTA_TIME: Nanotime =
    Nanotime::millis(1000 / PHYSICS_CONSTANT_UPDATE_RATE as i64);

pub fn simulate_vehicle(mut vehicle: Vehicle, gravity: Vec2) -> Vec<(Vec2, f32)> {
    let end = Nanotime::secs(30);
    let dt = PHYSICS_CONSTANT_DELTA_TIME;
    let mut ret = vec![];
    let mut t = Nanotime::zero();

    while t < end {
        t += dt;
        vehicle.step(gravity, dt);
        let pos = vehicle.pv.pos_f32();
        let angle = vehicle.angle();
        ret.push((pos, angle));
    }

    ret
}

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

#[derive(Debug, Clone)]
pub struct RigidBody {
    angle: f32,
    angular_velocity: f32,
}

impl RigidBody {
    fn step(&mut self, angular_accel: f32, dt: Nanotime) {
        self.angular_velocity += angular_accel * dt.to_secs();
        self.angular_velocity = self.angular_velocity.clamp(-2.0, 2.0);
        self.angle += self.angular_velocity * dt.to_secs();
        self.angle = wrap_0_2pi(self.angle);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartId(u64);

#[derive(Debug, Clone)]
pub struct Vehicle {
    name: String,

    pub pv: PV,
    pub policy: VehicleControlPolicy,

    body: RigidBody,
    pipes: HashSet<IVec2>,
    next_part_id: PartId,
    parts: HashMap<PartId, InstantiatedPart>,
    conn_groups: Vec<ConnectivityGroup>,
}

impl Vehicle {
    pub fn new() -> Self {
        Self::from_parts("".into(), Vec::new())
    }

    pub fn from_parts(name: String, prototypes: Vec<(IVec2, Rotation, PartPrototype)>) -> Self {
        let mut next_part_id = PartId(0);
        let mut parts = HashMap::new();

        for (pos, rot, proto) in prototypes {
            let instance = InstantiatedPart::from_prototype(proto, pos, rot);
            parts.insert(next_part_id, instance);

            next_part_id.0 += 1;
        }

        let mut ret = Self {
            pv: PV::ZERO,
            policy: VehicleControlPolicy::Idle,
            name,
            body: RigidBody {
                angle: rand(0.0, 2.0 * PI),
                angular_velocity: rand(-0.3, 0.3),
            },
            next_part_id,
            parts,
            pipes: HashSet::new(),
            conn_groups: Vec::new(),
        };

        ret.update();

        ret
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

    pub fn add_part(&mut self, proto: PartPrototype, pos: IVec2, rot: Rotation) {
        let id = self.get_next_part_id();
        let instance = InstantiatedPart::from_prototype(proto, pos, rot);
        self.parts.insert(id, instance);
        self.update();
    }

    pub fn get_part(&self, id: PartId) -> Option<&InstantiatedPart> {
        self.parts.get(&id)
    }

    pub fn get_part_at(&self, p: IVec2, layer: impl Into<Option<PartLayer>>) -> Option<PartId> {
        let layer: Option<PartLayer> = layer.into();

        for part_layer in enum_iterator::reverse_all::<PartLayer>() {
            let found = self.parts.iter().find(|(id, instance)| {
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

    pub fn remove_part_at(&mut self, p: IVec2, layer: impl Into<Option<PartLayer>>) {
        let layer: Option<PartLayer> = layer.into();
        self.parts.retain(|_, instance| {
            if let Some(focus) = layer {
                if instance.prototype().layer() != focus {
                    return true;
                }
            };
            let pixels = occupied_pixels(
                instance.origin(),
                instance.rotation(),
                &instance.prototype(),
            );
            !pixels.contains(&p)
        });
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
                    local_graph.connect(id, p);
                }

                for off in [IVec2::X, IVec2::Y, -IVec2::X, -IVec2::Y] {
                    let neighbor = p - off;
                    if self.pipes.contains(&neighbor) {
                        open_set.insert(neighbor);
                    }
                }
            }

            conn_groups.push(local_graph);
        }

        self.conn_groups = conn_groups;
    }

    pub fn conn_groups(&self) -> impl Iterator<Item = &ConnectivityGroup> + use<'_> {
        self.conn_groups.iter()
    }

    pub fn is_connected(&self, id_a: PartId, id_b: PartId) -> bool {
        self.conn_groups.iter().any(|g| g.is_connected(id_a, id_b))
    }

    fn update(&mut self) {
        self.construct_connectivity();
    }

    pub fn pipes(&self) -> impl Iterator<Item = IVec2> + use<'_> {
        self.pipes.iter().cloned()
    }

    pub fn parts(&self) -> impl Iterator<Item = (&PartId, &InstantiatedPart)> + use<'_> {
        self.parts.iter()
    }

    pub fn fuel_percentage(&self) -> f32 {
        let max_fuel_mass: Mass = self.tanks().map(|(t, d)| t.max_fluid_mass).sum();
        if max_fuel_mass == Mass::ZERO {
            return 0.0;
        }
        let current_fuel_mass: Mass = self.tanks().map(|(t, d)| t.stored(d)).sum();
        current_fuel_mass.to_kg_f32() / max_fuel_mass.to_kg_f32()
    }

    pub fn is_controllable(&self) -> bool {
        // TODO
        true
    }

    pub fn dry_mass(&self) -> Mass {
        self.current_mass() - self.fuel_mass()
    }

    pub fn fuel_mass(&self) -> Mass {
        self.tanks().map(|(t, d)| t.stored(d)).sum()
    }

    pub fn current_mass(&self) -> Mass {
        self.parts.iter().map(|(_, p)| p.current_mass()).sum()
    }

    pub fn thruster_count(&self) -> usize {
        self.thrusters().count()
    }

    pub fn tank_count(&self) -> usize {
        self.tanks().count()
    }

    pub fn thrust(&self) -> f32 {
        if self.thruster_count() == 0 {
            0.0
        } else {
            self.thrusters().map(|(t, _)| t.thrust).sum()
        }
    }

    pub fn max_thrust_along_heading(&self, angle: f32, rcs: bool) -> f32 {
        if self.thruster_count() == 0 {
            return 0.0;
        }

        let u = rotate(Vec2::X, angle);

        let mut sum = 0.0;

        for (_, part) in &self.parts {
            if let Some((t, _)) = part.as_thruster() {
                if t.is_rcs != rcs {
                    continue;
                }
                let v = rotate(Vec2::X, part.rotation().to_angle());
                let dot = u.dot(v).max(0.0);
                sum += dot * t.thrust;
            }
        }

        sum
    }

    pub fn center_of_mass(&self) -> Vec2 {
        let mass = self.current_mass();
        self.parts
            .iter()
            .map(|(_, p)| {
                let center = p.origin().as_vec2() / PIXELS_PER_METER + p.dims_meters() / 2.0;
                let weight = p.current_mass().to_kg_f32() / mass.to_kg_f32();
                center * weight
            })
            .sum()
    }

    pub fn accel(&self) -> f32 {
        let thrust = self.thrust();
        let mass = self.current_mass();
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
        self.thrusters().any(|(t, d)| t.is_thrusting(d))
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
        if self.current_mass() == Mass::ZERO || self.dry_mass() == Mass::ZERO {
            return 0.0;
        }
        let ve = self.average_linear_exhaust_velocity();
        rocket_equation(ve, self.current_mass(), self.dry_mass())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn current_angular_acceleration(&self) -> f32 {
        let mut aa = 0.0;
        let moa = self.current_mass().to_kg_f32(); // TODO
        let com = self.center_of_mass();

        for (_, part) in &self.parts {
            if let Some((t, d)) = part.as_thruster() {
                let center_of_thrust = part.center_meters();
                let lever_arm = center_of_thrust - com;
                let thrust_dir = rotate(Vec2::X, part.rotation().to_angle());
                let torque = cross2d(lever_arm, thrust_dir) * t.throttle(d) * t.thrust;
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
        let mass = self.current_mass().to_kg_f32();

        for (_, part) in &self.parts {
            if let Some((t, d)) = part.as_thruster() {
                let thrust_dir = rotate(Vec2::X, part.rotation().to_angle());
                body_frame_force += thrust_dir * t.thrust * d.throttle();
            }
        }

        body_frame_force / mass
    }

    fn step_thrust_control(&mut self, control: VehicleControl) {
        let com = self.center_of_mass();

        for (_, part) in &mut self.parts {
            let center_of_thrust = part.center_meters();
            let u = rotate(Vec2::X, part.rotation().to_angle());
            if let Some((t, d)) = part.as_thruster_mut() {
                let is_torque = t.is_rcs() && {
                    let torque = cross2d(center_of_thrust - com, u);
                    torque.signum() == control.attitude.signum() && control.attitude.abs() > 2.0
                };
                let is_linear =
                    t.is_rcs() == control.allow_linear_rcs && u.dot(control.linear) > 0.9;
                let throttle: f32 = if is_linear {
                    control.throttle
                } else if is_torque {
                    control.attitude.abs()
                } else {
                    0.0
                };
                d.set_throttle(throttle);
                d.on_sim_tick(t);
            }

            if let Some((m, d)) = part.as_magnetorquer_mut() {
                d.set_torque(m, 1000.0);
            }
        }
    }

    pub fn on_sim_tick(&mut self) {
        for (_, part) in &mut self.parts {
            if part.percent_built() < 1.0 {
                continue;
            }

            if let Some((t, d)) = part.as_thruster_mut() {
                d.on_sim_tick(t);
            }

            if let Some((_, d)) = part.as_machine_mut() {
                d.on_sim_tick();
            }

            if let Some((t, d)) = part.as_tank_mut() {
                let item = d.item().unwrap_or(Item::random());
                t.put(item, Mass::kilograms(20), d);
            }

            if let Some((c, d)) = part.as_cargo_mut() {
                let mass = randint(100, 700);
                c.put(Item::random(), Mass::kilograms(mass as u64), d);
            }
        }
    }

    pub fn step(&mut self, gravity: Vec2, dt: Nanotime) {
        let control = current_control_law(&self, gravity);
        self.step_thrust_control(control);

        let a = self.current_body_frame_linear_acceleration();
        let a = rotate(a, self.body.angle);

        let aa = self.current_angular_acceleration();
        // let fcr = self.fuel_consumption_rate();

        // let n = self.tank_count() as f32;

        self.body.step(aa, dt);

        let dv = (gravity + a) * dt.to_secs();

        self.pv.vel += dv.as_dvec2();
        self.pv.pos += self.pv.vel * dt.to_secs_f64();
    }

    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.body.angle)
    }

    pub fn angular_velocity(&self) -> f32 {
        self.body.angular_velocity
    }

    pub fn angle(&self) -> f32 {
        self.body.angle
    }

    pub fn kinematic_apoapis(&self, gravity: f64) -> f64 {
        if self.pv.vel.y <= 0.0 {
            return self.pv.pos.y;
        }
        self.pv.pos.y + self.pv.vel.y.powi(2) / (2.0 * gravity.abs())
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

    pub fn bounding_radius(&self) -> f32 {
        // TODO
        10.0
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
}
