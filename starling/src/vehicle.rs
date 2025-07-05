use crate::aabb::AABB;
use crate::aabb::OBB;
use crate::factory::Factory;
use crate::factory::Mass;
use crate::math::{cross2d, rand, rotate, IVec2, UVec2, Vec2, PI};
use crate::nanotime::Nanotime;
use crate::orbits::{wrap_0_2pi, wrap_pi_npi};
use crate::parts::{
    magnetorquer::Magnetorquer,
    parts::{PartDefinition, PartDefinitionVariant, PartLayer},
    radar::Radar,
    tank::Tank,
    thruster::Thruster,
};
use crate::pid::*;
use crate::pv::PV;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Sequence, Serialize, Deserialize)]
pub enum Rotation {
    East,
    North,
    West,
    South,
}

impl Rotation {
    pub fn to_angle(&self) -> f32 {
        match self {
            Self::East => 0.0,
            Self::North => PI * 0.5,
            Self::West => PI,
            Self::South => PI * 1.5,
        }
    }
}

fn rocket_equation(ve: f32, m0: Mass, m1: Mass) -> f32 {
    ve * (m0.to_kg_f32() / m1.to_kg_f32()).ln()
}

#[allow(unused)]
fn mass_after_maneuver(ve: f32, m0: f32, dv: f32) -> f32 {
    m0 / (dv / ve).exp()
}

pub fn pixel_dims_with_rotation(rot: Rotation, part: &PartDefinition) -> UVec2 {
    match rot {
        Rotation::East | Rotation::West => UVec2::new(part.width, part.height),
        Rotation::North | Rotation::South => UVec2::new(part.height, part.width),
    }
}

pub fn meters_with_rotation(rot: Rotation, part: &PartDefinition) -> Vec2 {
    let w = part.width_meters();
    let h = part.height_meters();
    match rot {
        Rotation::East | Rotation::West => Vec2::new(w, h),
        Rotation::North | Rotation::South => Vec2::new(h, w),
    }
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
        let px = 0.0; // position_error.dot(vehicle.pointing());
        let py = 0.0; // position_error.dot(rotate(vehicle.pointing(), PI / 2.0));

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

#[derive(Debug, Clone)]
pub struct PartInstance {
    builds_remaining: u32,
    origin: IVec2,
    rot: Rotation,
    proto: PartDefinition,
    variant: PartVariant,
}

impl PartInstance {
    pub fn new(origin: IVec2, rot: Rotation, proto: PartDefinition) -> Self {
        // TODO TODO TODO TODO
        Self {
            builds_remaining: 15,
            origin,
            rot,
            proto,
            variant: PartVariant::Other,
        }
    }

    pub fn build(&mut self) {
        if self.builds_remaining > 0 {
            self.builds_remaining -= 1;
        }
    }

    pub fn percent_built(&self) -> f32 {
        (1.0 - self.builds_remaining as f32 / 15.0).clamp(0.0, 1.0)
    }

    pub fn definition_variant(&self) -> &PartDefinitionVariant {
        &self.proto.class
    }

    pub fn sprite_path(&self) -> &str {
        &self.proto.path
    }

    pub fn sprite_dims(&self) -> UVec2 {
        UVec2::new(self.proto.width, self.proto.height)
    }

    pub fn dims_grid(&self) -> UVec2 {
        pixel_dims_with_rotation(self.rot, &self.proto)
    }

    pub fn dims_meters(&self) -> Vec2 {
        meters_with_rotation(self.rot, &self.proto)
    }

    pub fn origin(&self) -> IVec2 {
        self.origin
    }

    pub fn set_origin(&mut self, p: IVec2) {
        self.origin = p;
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

    pub fn layer(&self) -> PartLayer {
        self.proto.layer
    }

    pub fn mass(&self) -> Mass {
        self.proto.mass
    }

    pub fn proto(&self) -> &PartDefinition {
        &self.proto
    }
}

#[derive(Debug, Clone)]
pub struct Vehicle {
    name: String,
    pub pv: PV,
    pub policy: VehicleControlPolicy,
    stamp: Nanotime,
    angle: f32,
    angular_velocity: f32,
    factory: Factory,
    pub parts: Vec<PartInstance>,
}

#[derive(Debug, Clone)]
pub enum PartVariant {
    Structure,
    Thruster(Thruster),
    Magnetorquer(Magnetorquer),
    Radar(Radar),
    Tank(Tank),
    Cargo,
    Other,
}

impl Vehicle {
    pub fn from_parts(
        name: String,
        stamp: Nanotime,
        part_protos: Vec<(IVec2, Rotation, PartDefinition)>,
    ) -> Self {
        let mut parts: Vec<_> = part_protos
            .into_iter()
            .map(|(origin, rot, proto)| PartInstance::new(origin, rot, proto))
            .collect();

        parts.iter_mut().for_each(|instance| {
            let dims = meters_with_rotation(instance.rot, &instance.proto);
            instance.variant = match &instance.proto.class {
                PartDefinitionVariant::Thruster(proto) => PartVariant::Thruster(Thruster::new(
                    proto.clone(),
                    instance.origin.as_vec2() / crate::parts::parts::PIXELS_PER_METER + dims / 2.0,
                    instance.rot.to_angle() + PI / 2.0,
                )),
                PartDefinitionVariant::Cargo => PartVariant::Cargo,
                PartDefinitionVariant::Radar => PartVariant::Radar(Radar {}),
                PartDefinitionVariant::Tank(tank) => PartVariant::Tank(Tank {
                    pos: instance.origin.as_vec2() / crate::parts::parts::PIXELS_PER_METER,
                    width: dims.x,
                    height: dims.y,
                    item: tank.item,
                    dry_mass: instance.proto.mass,
                    current_fuel_mass: tank.wet_mass,
                    maximum_fuel_mass: tank.wet_mass,
                }),
                PartDefinitionVariant::Undefined => PartVariant::Other,
            };
        });

        let factory = Factory::new();

        Self {
            pv: PV::ZERO,
            policy: VehicleControlPolicy::Idle,
            name,
            stamp,
            angle: rand(0.0, 2.0 * PI),
            angular_velocity: rand(-0.3, 0.3),
            factory,
            parts,
        }
    }

    pub fn stamp(&self) -> Nanotime {
        self.stamp
    }

    pub fn factory(&self) -> &Factory {
        &self.factory
    }

    pub fn parts(&self) -> impl Iterator<Item = &PartInstance> + use<'_> {
        self.parts.iter()
    }

    pub fn fuel_percentage(&self) -> f32 {
        let max_fuel_mass: Mass = self.tanks().map(|t| t.maximum_fuel_mass).sum();
        if max_fuel_mass == Mass::ZERO {
            return 0.0;
        }
        let current_fuel_mass: Mass = self.tanks().map(|t| t.current_fuel_mass).sum();
        current_fuel_mass.to_kg_f32() / max_fuel_mass.to_kg_f32()
    }

    pub fn parts_by_layer(&self) -> impl Iterator<Item = &PartInstance> + use<'_> {
        // TODO this doesn't support all layers automatically!

        let dummy = PartLayer::Exterior;

        // compile error if I add more layers
        let _ = match dummy {
            PartLayer::Exterior => (),
            PartLayer::Internal => (),
            PartLayer::Structural => (),
        };

        let x = self
            .parts
            .iter()
            .filter(|instance| instance.proto.layer == PartLayer::Internal);
        let y = self
            .parts
            .iter()
            .filter(|instance| instance.proto.layer == PartLayer::Structural);
        let z = self
            .parts
            .iter()
            .filter(|instance| instance.proto.layer == PartLayer::Exterior);

        x.chain(y).chain(z)
    }

    pub fn is_controllable(&self) -> bool {
        !self.thrusters().count() == 0
    }

    pub fn dry_mass(&self) -> Mass {
        self.parts.iter().map(|instance| instance.proto.mass).sum()
    }

    pub fn fuel_mass(&self) -> Mass {
        self.tanks().map(|t: &Tank| t.current_fuel_mass).sum()
    }

    pub fn wet_mass(&self) -> Mass {
        self.dry_mass() + self.fuel_mass()
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
            self.thrusters().map(|t| t.model().thrust).sum()
        }
    }

    pub fn max_thrust_along_heading(&self, angle: f32, rcs: bool) -> f32 {
        if self.thruster_count() == 0 {
            return 0.0;
        }

        let u = rotate(Vec2::X, angle);

        self.thrusters()
            .map(|t| {
                if t.model().is_rcs != rcs {
                    return 0.0;
                }
                let v = t.pointing();
                let dot = u.dot(v);
                if dot < 0.9 {
                    0.0
                } else {
                    dot * t.model().thrust
                }
            })
            .sum()
    }

    pub fn center_of_mass(&self) -> Vec2 {
        Vec2::ZERO // TODO
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
            let dims = meters_with_rotation(instance.rot, &instance.proto);
            let pos = instance.origin.as_vec2() / crate::parts::parts::PIXELS_PER_METER;
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
            let upper = instance.origin + dims.as_ivec2();
            if let Some((min, max)) = min.as_mut().zip(max.as_mut()) {
                min.x = min.x.min(instance.origin.x);
                min.y = min.y.min(instance.origin.y);
                max.x = max.x.max(upper.x);
                max.y = max.y.max(upper.y);
            } else {
                min = Some(instance.origin);
                max = Some(upper);
            }
        }
        min.zip(max)
    }

    pub fn low_fuel(&self) -> bool {
        self.is_controllable() && self.remaining_dv() < 50.0
    }

    pub fn is_thrusting(&self) -> bool {
        self.thrusters().any(|t| t.is_thrusting())
    }

    pub fn has_radar(&self) -> bool {
        !self.radars().count() > 0
    }

    pub fn average_linear_exhaust_velocity(&self) -> f32 {
        let linear_thrusters: Vec<_> = self.thrusters().filter(|t| !t.is_rcs()).collect();

        let count = linear_thrusters.len();

        if count == 0 {
            return 0.0;
        }

        linear_thrusters
            .into_iter()
            .map(|t| t.model().exhaust_velocity / count as f32)
            .sum()
    }

    pub fn body_frame_acceleration(&self) -> Vec2 {
        let mass = self.wet_mass();
        self.thrusters()
            .map(|t| t.thrust_vector() / mass.to_kg_f32())
            .sum()
    }

    pub fn fuel_consumption_rate(&self) -> f32 {
        self.thrusters().map(|t| t.fuel_consumption_rate()).sum()
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

        self.thrusters().filter(|t| t.is_thrusting()).for_each(|t| {
            let lever_arm = t.pos - com;
            let torque = cross2d(lever_arm, t.pointing()) * t.throttle() * t.model().thrust;
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
        self.thrusters().filter(|t| t.is_thrusting()).for_each(|t| {
            a += rotate(t.pointing(), self.angle) * t.model().thrust / mass * t.throttle();
        });
        a
    }

    fn step_thrust_control(&mut self, stamp: Nanotime, control: VehicleControl) {
        if !self.is_controllable() {
            return;
        }

        if self.remaining_dv() == 0.0 {
            self.set_zero_thrust(stamp);
            return;
        }

        for t in &mut self.thrusters_mut() {
            let u = t.pointing();
            let is_torque = t.is_rcs() && {
                let torque = cross2d(t.pos, u);
                torque.signum() == control.attitude.signum() && control.attitude.abs() > 2.0
            };
            let is_linear = t.is_rcs() == control.allow_linear_rcs && u.dot(control.linear) > 0.9;
            let throttle = if is_linear {
                control.throttle
            } else if is_torque {
                control.attitude.abs()
            } else {
                0.0
            };
            t.set_thrusting(throttle, stamp);
        }

        for t in &mut self.magnetorquers_mut() {
            t.set_torque(control.attitude * 100.0);
        }
    }

    fn set_zero_thrust(&mut self, stamp: Nanotime) {
        for t in self.thrusters_mut() {
            t.set_thrusting(0.0, stamp);
        }
        for t in self.magnetorquers_mut() {
            t.current_torque = 0.0;
        }
    }

    fn step_physics(&mut self, gravity: Vec2) {
        const NOMINAL_DT: Nanotime = Nanotime::millis(20);

        let a = self.current_linear_acceleration();
        let aa = self.current_angular_acceleration();
        let fcr = self.fuel_consumption_rate();

        let n: f32 = self.tank_count() as f32;
        for t in self.tanks_mut() {
            t.current_fuel_mass -= Mass::from_kg_f32(fcr * NOMINAL_DT.to_secs() / n);
            t.current_fuel_mass = t.current_fuel_mass.clamp(Mass::ZERO, t.maximum_fuel_mass);
        }

        self.angular_velocity += aa * NOMINAL_DT.to_secs();

        self.angular_velocity = self.angular_velocity.clamp(-2.0, 2.0);

        self.angle += self.angular_velocity * NOMINAL_DT.to_secs();
        self.angle = wrap_0_2pi(self.angle);
        self.stamp += NOMINAL_DT;

        let dv = (gravity + a) * NOMINAL_DT.to_secs();

        self.pv.vel += dv.as_dvec2();
        self.pv.pos += self.pv.vel * NOMINAL_DT.to_secs_f64();
    }

    pub fn step(&mut self, stamp: Nanotime, mode: PhysicsMode, gravity: Vec2) {
        if self.remaining_dv() == 0.0 {
            self.stamp = stamp;
        }

        while self.stamp < stamp {
            let control = current_control_law(&self, gravity);

            match mode {
                PhysicsMode::Limited => self.set_zero_thrust(self.stamp),
                PhysicsMode::RealTime => self.step_thrust_control(self.stamp, control),
            };

            self.step_physics(gravity);
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
            if let PartVariant::Radar(r) = &instance.variant {
                Some(r)
            } else {
                None
            }
        })
    }

    pub fn magnetorquers(&self) -> impl Iterator<Item = &Magnetorquer> + use<'_> {
        self.parts.iter().filter_map(|instance| {
            if let PartVariant::Magnetorquer(m) = &instance.variant {
                Some(m)
            } else {
                None
            }
        })
    }

    pub fn magnetorquers_mut(&mut self) -> impl Iterator<Item = &mut Magnetorquer> + use<'_> {
        self.parts.iter_mut().filter_map(|instance| {
            if let PartVariant::Magnetorquer(m) = &mut instance.variant {
                Some(m)
            } else {
                None
            }
        })
    }

    pub fn thrusters(&self) -> impl Iterator<Item = &Thruster> + use<'_> {
        self.parts.iter().filter_map(|instance| {
            if let PartVariant::Thruster(t) = &instance.variant {
                Some(t)
            } else {
                None
            }
        })
    }

    pub fn thrusters_mut(&mut self) -> impl Iterator<Item = &mut Thruster> + use<'_> {
        self.parts.iter_mut().filter_map(|instance| {
            if let PartVariant::Thruster(t) = &mut instance.variant {
                Some(t)
            } else {
                None
            }
        })
    }

    pub fn tanks(&self) -> impl Iterator<Item = &Tank> + use<'_> {
        self.parts.iter().filter_map(|instance| {
            if let PartVariant::Tank(t) = &instance.variant {
                Some(t)
            } else {
                None
            }
        })
    }

    pub fn tanks_mut(&mut self) -> impl Iterator<Item = &mut Tank> + use<'_> {
        self.parts.iter_mut().filter_map(|instance| {
            if let PartVariant::Tank(t) = &mut instance.variant {
                Some(t)
            } else {
                None
            }
        })
    }

    pub fn bounding_radius(&self) -> f32 {
        // TODO
        100.0
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
