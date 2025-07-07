use crate::aabb::AABB;
use crate::factory::Mass;
use crate::math::*;
use crate::nanotime::Nanotime;
use crate::orbits::{wrap_0_2pi, wrap_pi_npi};
use crate::parts::*;
use crate::pid::*;
use crate::pv::PV;
use crate::vehicle::*;
use std::collections::HashSet;

fn rocket_equation(ve: f32, m0: Mass, m1: Mass) -> f32 {
    ve * (m0.to_kg_f32() / m1.to_kg_f32()).ln()
}

#[allow(unused)]
fn mass_after_maneuver(ve: f32, m0: f32, dv: f32) -> f32 {
    m0 / (dv / ve).exp()
}

#[derive(Debug, Clone, Copy)]
pub enum PhysicsMode {
    RealTime,
    Limited,
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
    PlayerControlled,
    PositionHold(Vec2),
}

const ATTITUDE_CONTROLLER: PDCtrl = PDCtrl::new(30.0, 35.0);

const VERTICAL_CONTROLLER: PDCtrl = PDCtrl::new(0.2, 1.0);

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

    let horizontal_control = {
        // horizontal controller
        let kp = 0.01;
        let kd = 0.08;

        // positive means to the right, which corresponds to a negative heading correction
        kp * (target.x - vehicle.pv.pos.x as f32) - kd * vehicle.pv.vel.x as f32
    };

    // attitude controller
    let target_angle = PI * 0.5 - horizontal_control.clamp(-PI / 4.0, PI / 4.0);
    let attitude_error = wrap_pi_npi(target_angle - vehicle.angle());
    let attitude = compute_attitude_control(vehicle, target_angle, &ATTITUDE_CONTROLLER);

    let thrust = vehicle.max_thrust_along_heading(0.0, false);
    let accel = thrust / vehicle.wet_mass().to_kg_f32();
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
    if gravity.length() > 0.0 {
        hover_control_law(gravity, vehicle)
    } else {
        zero_gravity_control_law(vehicle)
    }
}

pub fn simulate_vehicle(mut vehicle: Vehicle, gravity: Vec2) -> Vec<(Vec2, f32)> {
    let start = vehicle.stamp();
    let end = start + Nanotime::secs(30);
    let dt = Nanotime::millis(50);

    let mut ret = vec![];

    let mut t = start;

    while t < end {
        t += dt;
        vehicle.step(t, PhysicsMode::RealTime, gravity);
        let pos = vehicle.pv.pos_f32();
        let angle = vehicle.angle();
        ret.push((pos, angle));
    }

    ret
}

pub fn occupied_pixels(pos: IVec2, rot: Rotation, part: &Part) -> Vec<IVec2> {
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
pub struct Vehicle {
    name: String,
    pub pv: PV,
    pub policy: VehicleControlPolicy,
    stamp: Nanotime,
    angle: f32,
    angular_velocity: f32,
    pipes: HashSet<IVec2>,
    parts: Vec<PartInstance>,
    conn_groups: Vec<ConnectivityGroup>,
}

impl Vehicle {
    pub fn from_parts(name: String, stamp: Nanotime, parts: Vec<(IVec2, Rotation, Part)>) -> Self {
        let instances: Vec<_> = parts
            .into_iter()
            .map(|(origin, rot, part)| PartInstance::new(origin, rot, part))
            .collect();

        let mut ret = Self {
            pv: PV::ZERO,
            policy: VehicleControlPolicy::Idle,
            name,
            stamp,
            angle: rand(0.0, 2.0 * PI),
            angular_velocity: rand(-0.3, 0.3),
            parts: instances,
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

    pub fn add_part(&mut self, instance: PartInstance) {
        self.parts.push(instance);
        self.update();
    }

    pub fn get_part_by_index(&self, idx: usize) -> Option<&PartInstance> {
        self.parts.get(idx)
    }

    pub fn get_part_at(
        &self,
        p: IVec2,
        layer: impl Into<Option<PartLayer>>,
    ) -> Option<(usize, &PartInstance)> {
        let layer: Option<PartLayer> = layer.into();
        self.parts.iter().enumerate().find(|(_, instance)| {
            if let Some(layer) = layer {
                if layer != instance.part().layer() {
                    return false;
                }
            }
            let origin = instance.origin();
            let dims = instance.dims_grid().as_ivec2();
            let p = p - origin;
            p.x >= 0 && p.y >= 0 && p.x <= dims.x && p.y <= dims.y
        })
    }

    pub fn remove_part_at(&mut self, p: IVec2, layer: impl Into<Option<PartLayer>>) {
        let layer: Option<PartLayer> = layer.into();
        self.parts.retain(|instance| {
            if let Some(focus) = layer {
                if instance.part().layer() != focus {
                    return true;
                }
            };
            let pixels = occupied_pixels(instance.origin(), instance.rotation(), instance.part());
            !pixels.contains(&p)
        });
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

                if let Some((idx, _)) = self.get_part_at(p, PartLayer::Internal) {
                    local_graph.connect(idx, p);
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

    pub fn is_connected(&self, idx_a: usize, idx_b: usize) -> bool {
        self.conn_groups
            .iter()
            .any(|g| g.is_connected(idx_a, idx_b))
    }

    fn update(&mut self) {
        self.construct_connectivity();
    }

    pub fn pipes(&self) -> impl Iterator<Item = IVec2> + use<'_> {
        self.pipes.iter().cloned()
    }

    pub fn stamp(&self) -> Nanotime {
        self.stamp
    }

    pub fn parts(&self) -> impl Iterator<Item = &PartInstance> + use<'_> {
        self.parts.iter()
    }

    pub fn fuel_percentage(&self) -> f32 {
        let max_fuel_mass: Mass = self.tanks().map(|t| t.max_fluid_mass).sum();
        if max_fuel_mass == Mass::ZERO {
            return 0.0;
        }
        let current_fuel_mass: Mass = self.tanks().map(|t| t.stored()).sum();
        current_fuel_mass.to_kg_f32() / max_fuel_mass.to_kg_f32()
    }

    pub fn is_controllable(&self) -> bool {
        !self.thrusters_ref().count() == 0
    }

    pub fn dry_mass(&self) -> Mass {
        self.parts
            .iter()
            .map(|instance| instance.part().dry_mass())
            .sum()
    }

    pub fn fuel_mass(&self) -> Mass {
        self.tanks().map(|t: &Tank| t.stored()).sum()
    }

    pub fn wet_mass(&self) -> Mass {
        self.dry_mass() + self.fuel_mass()
    }

    pub fn thruster_count(&self) -> usize {
        self.thrusters_ref().count()
    }

    pub fn tank_count(&self) -> usize {
        self.tanks().count()
    }

    pub fn thrust(&self) -> f32 {
        if self.thruster_count() == 0 {
            0.0
        } else {
            self.thrusters_ref().map(|t| t.variant.model().thrust).sum()
        }
    }

    pub fn max_thrust_along_heading(&self, angle: f32, rcs: bool) -> f32 {
        if self.thruster_count() == 0 {
            return 0.0;
        }

        let u = rotate(Vec2::X, angle);

        self.thrusters_ref()
            .map(|t| {
                if t.variant.model().is_rcs != rcs {
                    return 0.0;
                }
                let v = t.thrust_pointing();
                let dot = u.dot(v);
                if dot < 0.9 {
                    0.0
                } else {
                    dot * t.variant.model().thrust
                }
            })
            .sum()
    }

    pub fn center_of_mass(&self) -> Vec2 {
        let mass = self.wet_mass();
        self.parts
            .iter()
            .map(|p| {
                let center = p.origin().as_vec2() / PIXELS_PER_METER + p.dims_meters() / 2.0;
                let weight = p.part().current_mass().to_kg_f32() / mass.to_kg_f32();
                center * weight
            })
            .sum()
    }

    pub fn accel(&self) -> f32 {
        let thrust = self.thrust();
        let mass = self.wet_mass();
        if mass == Mass::ZERO {
            0.0
        } else {
            thrust / mass.to_kg_f32()
        }
    }

    pub fn aabb(&self) -> AABB {
        let mut ret: Option<AABB> = None;
        for instance in &self.parts {
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
        for instance in &self.parts {
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
        self.thrusters_ref().any(|t| t.variant.is_thrusting())
    }

    pub fn has_radar(&self) -> bool {
        !self.radars().count() > 0
    }

    pub fn average_linear_exhaust_velocity(&self) -> f32 {
        let linear_thrusters: Vec<_> = self
            .thrusters_ref()
            .filter(|t| !t.variant.is_rcs())
            .collect();

        let count = linear_thrusters.len();

        if count == 0 {
            return 0.0;
        }

        linear_thrusters
            .into_iter()
            .map(|t| t.variant.model().exhaust_velocity / count as f32)
            .sum()
    }

    pub fn body_frame_acceleration(&self) -> Vec2 {
        let mass = self.wet_mass();
        self.thrusters_ref()
            .map(|t| t.variant.thrust_vector() / mass.to_kg_f32())
            .sum()
    }

    pub fn fuel_consumption_rate(&self) -> f32 {
        self.thrusters_ref()
            .map(|t| t.variant.fuel_consumption_rate())
            .sum()
    }

    pub fn remaining_dv(&self) -> f32 {
        if self.wet_mass() == Mass::ZERO || self.dry_mass() == Mass::ZERO {
            return 0.0;
        }
        let ve = self.average_linear_exhaust_velocity();
        rocket_equation(ve, self.wet_mass(), self.dry_mass())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn current_angular_acceleration(&self) -> f32 {
        let mut aa = 0.0;
        let moa = self.wet_mass().to_kg_f32(); // TODO
        let com = self.center_of_mass();

        self.thrusters_ref()
            .filter(|t| t.variant.is_thrusting())
            .for_each(|t| {
                let center_of_thrust = t.center_meters();
                let lever_arm = center_of_thrust - com;
                let torque = cross2d(lever_arm, t.thrust_pointing())
                    * t.variant.throttle()
                    * t.variant.model().thrust;
                aa += torque / moa;
            });

        for t in self.magnetorquers() {
            aa += t.current_torque / moa;
        }
        aa
    }

    fn current_linear_acceleration(&self) -> Vec2 {
        let mut a = Vec2::ZERO;
        let mass = self.wet_mass().to_kg_f32();
        self.thrusters_ref()
            .filter(|t| t.variant.is_thrusting())
            .for_each(|t| {
                a += rotate(t.thrust_pointing(), self.angle) * t.variant.model().thrust / mass
                    * t.variant.throttle();
            });
        a
    }

    fn step_thrust_control(&mut self, stamp: Nanotime, control: VehicleControl) {
        // if !self.is_controllable() {
        //     return;
        // }

        // if self.remaining_dv() == 0.0 {
        //     self.set_zero_thrust(stamp);
        //     return;
        // }

        let com = self.center_of_mass();

        for t in self.thrusters_ref_mut() {
            let center_of_thrust = t.center_meters();
            let u = t.thrust_pointing();
            let is_torque = t.variant.is_rcs() && {
                let torque = cross2d(center_of_thrust - com, u);
                torque.signum() == control.attitude.signum() && control.attitude.abs() > 2.0
            };
            let is_linear =
                t.variant.is_rcs() == control.allow_linear_rcs && u.dot(control.linear) > 0.9;
            let throttle = if is_linear {
                control.throttle
            } else if is_torque {
                control.attitude.abs()
            } else {
                0.0
            };
            t.variant.set_thrusting(throttle, stamp);
        }

        for t in &mut self.magnetorquers_mut() {
            t.set_torque(control.attitude * 100.0);
        }
    }

    fn set_zero_thrust(&mut self, stamp: Nanotime) {
        for t in self.thrusters_ref_mut() {
            t.variant.set_thrusting(0.0, stamp);
        }
        for t in self.magnetorquers_mut() {
            t.current_torque = 0.0;
        }
    }

    fn step_physics(&mut self, gravity: Vec2, dt: Nanotime) {
        let a = self.current_linear_acceleration();
        let aa = self.current_angular_acceleration();
        let fcr = self.fuel_consumption_rate();

        let n: f32 = self.tank_count() as f32;
        for t in self.tanks_mut() {
            let delta_mass = Mass::from_kg_f32(fcr * dt.to_secs() / n);
            t.take(delta_mass);
        }

        self.angular_velocity += aa * dt.to_secs();

        self.angular_velocity = self.angular_velocity.clamp(-2.0, 2.0);

        self.angle += self.angular_velocity * dt.to_secs();
        self.angle = wrap_0_2pi(self.angle);
        self.stamp += dt;

        let dv = (gravity + a) * dt.to_secs();

        self.pv.vel += dv.as_dvec2();
        self.pv.pos += self.pv.vel * dt.to_secs_f64();
    }

    pub fn step(&mut self, stamp: Nanotime, mode: PhysicsMode, gravity: Vec2) {
        // if self.remaining_dv() == 0.0 {
        //     self.stamp = stamp;
        // }

        const NOMINAL_DT: Nanotime = Nanotime::millis(10);

        while self.stamp < stamp {
            let control = current_control_law(&self, gravity);

            match mode {
                PhysicsMode::Limited => self.set_zero_thrust(self.stamp),
                PhysicsMode::RealTime => self.step_thrust_control(self.stamp, control),
            };

            self.step_physics(gravity, NOMINAL_DT.min(stamp - self.stamp));
        }
    }

    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.angle)
    }

    pub fn angular_velocity(&self) -> f32 {
        self.angular_velocity
    }

    pub fn angle(&self) -> f32 {
        self.angle
    }

    pub fn kinematic_apoapis(&self, gravity: f64) -> f64 {
        if self.pv.vel.y <= 0.0 {
            return self.pv.pos.y;
        }
        self.pv.pos.y + self.pv.vel.y.powi(2) / (2.0 * gravity.abs())
    }

    pub fn radars(&self) -> impl Iterator<Item = &Radar> + use<'_> {
        self.parts.iter().filter_map(|instance| {
            if let Part::Radar(r) = &instance.part() {
                Some(r)
            } else {
                None
            }
        })
    }

    pub fn magnetorquers(&self) -> impl Iterator<Item = &Magnetorquer> + use<'_> {
        self.parts.iter().filter_map(|instance| {
            if let Part::Magnetorquer(m) = instance.part() {
                Some(m)
            } else {
                None
            }
        })
    }

    pub fn magnetorquers_mut(&mut self) -> impl Iterator<Item = &mut Magnetorquer> + use<'_> {
        self.parts.iter_mut().filter_map(|instance| {
            if let Part::Magnetorquer(m) = instance.part_mut() {
                Some(m)
            } else {
                None
            }
        })
    }

    pub fn thrusters_ref(&self) -> impl Iterator<Item = InstanceRef<&Thruster>> + use<'_> {
        self.parts.iter().filter_map(|p| {
            let pos = p.part().dims();
            let origin = p.origin();
            let rot = p.rotation();
            if let Part::Thruster(t) = p.part() {
                Some(InstanceRef::new(origin, pos, rot, t))
            } else {
                None
            }
        })
    }

    pub fn thrusters_ref_mut(
        &mut self,
    ) -> impl Iterator<Item = InstanceRef<&mut Thruster>> + use<'_> {
        self.parts.iter_mut().filter_map(|p| {
            let pos = p.part().dims();
            let origin = p.origin();
            let rot = p.rotation();
            if let Part::Thruster(t) = p.part_mut() {
                Some(InstanceRef::new(origin, pos, rot, t))
            } else {
                None
            }
        })
    }

    pub fn tanks(&self) -> impl Iterator<Item = &Tank> + use<'_> {
        self.parts.iter().filter_map(|instance| {
            if let Part::Tank(t) = instance.part() {
                Some(t)
            } else {
                None
            }
        })
    }

    pub fn tanks_mut(&mut self) -> impl Iterator<Item = &mut Tank> + use<'_> {
        self.parts.iter_mut().filter_map(|instance| {
            if let Part::Tank(t) = instance.part_mut() {
                Some(t)
            } else {
                None
            }
        })
    }

    pub fn bounding_radius(&self) -> f32 {
        // TODO
        10.0
    }

    pub fn build_once(&mut self) {
        for instance in &mut self.parts {
            if instance.percent_built() < 1.0 {
                instance.build();
                return;
            }
        }
    }
}
