use crate::aabb::AABB;
use crate::inventory::{Inventory, InventoryItem};
use crate::math::{cross2d, rand, randint, rotate, IVec2, UVec2, Vec2, PI};
use crate::nanotime::Nanotime;
use crate::orbits::{wrap_0_2pi, wrap_pi_npi};
use crate::parts::{
    magnetorquer::Magnetorquer,
    parts::{PartClass, PartLayer, PartProto},
    radar::Radar,
    tank::Tank,
    thruster::Thruster,
};
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

fn rocket_equation(ve: f32, m0: f32, m1: f32) -> f32 {
    ve * (m0 / m1).ln()
}

fn mass_after_maneuver(ve: f32, m0: f32, dv: f32) -> f32 {
    m0 / (dv / ve).exp()
}

fn random_sat_inventory() -> Inventory {
    use InventoryItem::*;
    let mut inv = Inventory::new();
    inv.add(Copper, randint(2000, 5000) as u64);
    inv.add(Silicon, randint(40, 400) as u64);
    inv.add(LiquidFuel, randint(500, 800) as u64 * 1000);
    inv
}

pub fn dims_with_rotation(rot: Rotation, part: &PartProto) -> UVec2 {
    match rot {
        Rotation::East | Rotation::West => UVec2::new(part.width, part.height),
        Rotation::North | Rotation::South => UVec2::new(part.height, part.width),
    }
}

pub fn meters_with_rotation(rot: Rotation, part: &PartProto) -> Vec2 {
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

#[derive(Debug, Clone, Copy)]
pub enum VehicleController {
    None,
    Attitude(f32),
    External,
}

#[derive(Debug, Clone)]
pub struct Vehicle {
    name: String,
    stamp: Nanotime,
    angle: f32,
    ctrl: VehicleController,
    angular_velocity: f32,
    thrusters: Vec<Thruster>,
    magnetorquers: Vec<Magnetorquer>,
    radars: Vec<Radar>,
    tanks: Vec<Tank>,
    bounding_radius: f32,
    center_of_mass: Vec2,
    pub inventory: Inventory,
    pub max_fuel_mass: f32,
    pub dry_mass: f32,
    pub exhaust_velocity: f32,
    pub parts: Vec<(IVec2, Rotation, PartProto)>,
    pub velocity_stamp: Nanotime,
    pub stored_delta_velocity: Vec2,
}

impl Vehicle {
    pub fn from_parts(
        name: String,
        stamp: Nanotime,
        parts: Vec<(IVec2, Rotation, PartProto)>,
    ) -> Self {
        let thrusters: Vec<Thruster> = parts
            .iter()
            .filter_map(|(pos, rot, p)| {
                let dims = meters_with_rotation(*rot, p);
                if let PartClass::Thruster(proto) = &p.data.class {
                    Some(Thruster::new(
                        proto.clone(),
                        pos.as_vec2() / crate::parts::parts::PIXELS_PER_METER + dims / 2.0,
                        rot.to_angle() + PI / 2.0,
                    ))
                } else {
                    None
                }
            })
            .collect();

        let dry_mass = parts.iter().map(|(_, _, p)| p.data.mass).sum();

        let linear_thrusters = thrusters.iter().filter(|t| !t.proto.is_rcs);
        let n_linear = linear_thrusters.clone().count();

        let isp = if n_linear == 0 {
            100.0
        } else {
            linear_thrusters.map(|t| t.proto.isp).sum::<f32>() / n_linear as f32
        };

        let tanks: Vec<Tank> = parts
            .iter()
            .filter_map(|(_, _, p)| {
                if let PartClass::Tank(proto) = p.data.class {
                    Some(Tank {
                        proto,
                        fuel_mass: (proto.wet_mass - p.data.mass),
                    })
                } else {
                    None
                }
            })
            .collect();

        let radars: Vec<Radar> = parts
            .iter()
            .filter_map(|(_, _, p)| {
                if let PartClass::Radar = p.data.class {
                    Some(Radar {})
                } else {
                    None
                }
            })
            .collect();

        let mut bounding_radius = 1.0;
        for (pos, rot, part) in &parts {
            let pos = pos.as_vec2() / crate::parts::parts::PIXELS_PER_METER;
            let dims = meters_with_rotation(*rot, part);
            let bounds = AABB::from_arbitrary(pos, pos + dims);
            for corners in bounds.corners() {
                let d = corners.length();
                if d > bounding_radius {
                    bounding_radius = d;
                }
            }
        }

        let center_of_mass = if parts.is_empty() {
            Vec2::ZERO
        } else {
            let sum: Vec2 = parts
                .iter()
                .map(|(pos, rot, part)| {
                    let dims = meters_with_rotation(*rot, part);
                    pos.as_vec2() / crate::parts::parts::PIXELS_PER_METER + dims / 2.0
                })
                .sum();

            sum / parts.len() as f32
        };

        Self {
            max_fuel_mass: 0.0,
            dry_mass,
            exhaust_velocity: isp * 9.81,
            name,
            stamp,
            angle: rand(0.0, 2.0 * PI),
            ctrl: VehicleController::Attitude(rand(0.0, PI * 2.0)),
            angular_velocity: rand(-0.3, 0.3),
            tanks,
            thrusters,
            radars,
            magnetorquers: vec![Magnetorquer {
                max_torque: 10000.0,
                current_torque: 0.0,
            }],
            inventory: random_sat_inventory(),
            parts,
            bounding_radius,
            center_of_mass,
            velocity_stamp: Nanotime::zero(),
            stored_delta_velocity: Vec2::ZERO,
        }
    }

    pub fn parts_by_layer(&self) -> impl Iterator<Item = &(IVec2, Rotation, PartProto)> + use<'_> {
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
            .filter(|(_, _, part)| part.data.layer == PartLayer::Internal);
        let y = self
            .parts
            .iter()
            .filter(|(_, _, part)| part.data.layer == PartLayer::Structural);
        let z = self
            .parts
            .iter()
            .filter(|(_, _, part)| part.data.layer == PartLayer::Exterior);

        y.chain(x).chain(z)
    }

    pub fn is_controllable(&self) -> bool {
        !self.thrusters.is_empty()
    }

    pub fn fuel_mass(&self) -> f32 {
        self.tanks.iter().map(|t| t.fuel_mass).sum()
    }

    pub fn wet_mass(&self) -> f32 {
        self.dry_mass + self.fuel_mass()
    }

    pub fn thruster_count(&self) -> usize {
        self.thrusters.len()
    }

    pub fn tank_count(&self) -> usize {
        self.tanks.len()
    }

    pub fn thrust(&self) -> f32 {
        if self.thrusters.is_empty() {
            0.0
        } else {
            self.thrusters.iter().map(|t| t.proto.thrust).sum()
        }
    }

    pub fn max_thrust_along_heading(&self, angle: f32, rcs: bool) -> f32 {
        if self.thrusters.is_empty() {
            return 0.0;
        }

        let u = rotate(Vec2::X, angle);

        self.thrusters
            .iter()
            .map(|t| {
                if t.proto.is_rcs != rcs {
                    return 0.0;
                }
                let v = t.pointing();
                let dot = u.dot(v);
                if dot < 0.9 {
                    0.0
                } else {
                    dot * t.proto.thrust
                }
            })
            .sum()
    }

    pub fn center_of_mass(&self) -> Vec2 {
        self.center_of_mass
    }

    pub fn accel(&self) -> f32 {
        let thrust = self.thrust();
        let mass = self.wet_mass();
        if mass == 0.0 {
            0.0
        } else {
            thrust / mass
        }
    }

    pub fn aabb(&self) -> AABB {
        let mut ret: Option<AABB> = None;
        for (pos, rot, part) in &self.parts {
            let dims = meters_with_rotation(*rot, part);
            let pos = pos.as_vec2() / crate::parts::parts::PIXELS_PER_METER;
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
        for (pos, rot, part) in &self.parts {
            let dims = dims_with_rotation(*rot, part);
            let upper = pos + dims.as_ivec2();
            if let Some((min, max)) = min.as_mut().zip(max.as_mut()) {
                min.x = min.x.min(pos.x);
                min.y = min.y.min(pos.y);
                max.x = max.x.max(upper.x);
                max.y = max.y.max(upper.y);
            } else {
                min = Some(*pos);
                max = Some(upper);
            }
        }
        min.zip(max)
    }

    pub fn low_fuel(&self) -> bool {
        self.is_controllable() && self.remaining_dv() < 10.0
    }

    pub fn is_thrusting(&self) -> bool {
        self.thrusters.iter().any(|t| t.is_thrusting())
    }

    pub fn has_radar(&self) -> bool {
        !self.radars.is_empty()
    }

    pub fn try_impulsive_burn(&mut self, dv: Vec2) -> Option<()> {
        if dv.length() > self.remaining_dv() {
            return None;
        }

        let fuel_mass_before_maneuver = self.fuel_mass();
        let m1 = mass_after_maneuver(self.exhaust_velocity, self.wet_mass(), dv.length());
        let fuel_mass_after_maneuver = m1 - self.dry_mass;
        let spent_fuel = fuel_mass_before_maneuver - fuel_mass_after_maneuver;

        self.inventory.take(
            InventoryItem::LiquidFuel,
            (spent_fuel * 1000.0).round() as u64,
        );

        Some(())
    }

    pub fn remaining_dv(&self) -> f32 {
        rocket_equation(self.exhaust_velocity, self.wet_mass(), self.dry_mass)
    }

    pub fn fuel_percentage(&self) -> f32 {
        self.fuel_mass() / self.max_fuel_mass
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn current_angular_acceleration(&self) -> f32 {
        let mut aa = 0.0;
        let moa = self.wet_mass(); // TODO
        let com = self.center_of_mass();
        for t in &self.thrusters {
            let lever_arm = t.pos - com;
            let torque = cross2d(lever_arm, t.pointing()) * t.throttle() * t.proto.thrust;
            aa += torque / moa;
        }
        for t in &self.magnetorquers {
            aa += t.current_torque / moa;
        }
        aa
    }

    fn current_linear_acceleration(&self) -> Vec2 {
        let mut a = Vec2::ZERO;
        let mass = self.wet_mass();
        for t in &self.thrusters {
            a += rotate(t.pointing(), self.angle) * t.proto.thrust / mass * t.throttle();
        }
        a
    }

    fn step_thrust_control(&mut self, stamp: Nanotime, control: Vec2, throttle: f32, is_rcs: bool) {
        if self.is_controllable() {
            if let VehicleController::Attitude(target_angle) = &mut self.ctrl {
                *target_angle = wrap_0_2pi(*target_angle);
                let kp = 20.0;
                let kd = 40.0;

                let error =
                    kp * wrap_pi_npi(*target_angle - self.angle) - kd * self.angular_velocity;

                for t in &mut self.thrusters {
                    let u = t.pointing();
                    let is_torque = t.proto.is_rcs && {
                        let torque = cross2d(t.pos, u);
                        torque.signum() == error.signum() && error.abs() > 6.0
                    };
                    let is_linear = t.proto.is_rcs == is_rcs && u.dot(control) > 0.9;
                    let throttle = if is_linear {
                        throttle
                    } else if is_torque {
                        error.abs()
                    } else {
                        0.0
                    };
                    t.set_thrusting(throttle, stamp);
                }
                for t in &mut self.magnetorquers {
                    t.set_torque(error * 1000.0);
                }
            }
        } else {
            self.ctrl = VehicleController::None;
        }
    }

    fn set_zero_thrust(&mut self, stamp: Nanotime) {
        for t in &mut self.thrusters {
            t.set_thrusting(0.0, stamp);
        }
        for t in &mut self.magnetorquers {
            t.current_torque = 0.0;
        }
    }

    fn iter_physics(&mut self, stamp: Nanotime) {
        let dt = stamp - self.stamp;

        let a = self.current_linear_acceleration();
        let aa = self.current_angular_acceleration();

        self.angular_velocity += aa * dt.to_secs();

        self.angular_velocity = self.angular_velocity.clamp(-2.0, 2.0);

        self.angle += self.angular_velocity * dt.to_secs();
        self.angle = wrap_0_2pi(self.angle);
        self.stamp = stamp;

        let dv = a * dt.to_secs();

        if dv.length() > 0.0 {
            if self.stored_delta_velocity.length() > 0.0 {
                self.velocity_stamp = stamp;
            }
            self.stored_delta_velocity += dv;
        }
    }

    pub fn step(
        &mut self,
        stamp: Nanotime,
        control: Vec2,
        throttle: f32,
        is_rcs: bool,
        mode: PhysicsMode,
    ) -> Vec2 {
        match mode {
            PhysicsMode::Limited => self.set_zero_thrust(stamp),
            PhysicsMode::RealTime => self.step_thrust_control(stamp, control, throttle, is_rcs),
        };

        self.iter_physics(stamp);

        let dt = stamp - self.velocity_stamp;

        if self.velocity_stamp != Nanotime::zero() && dt > Nanotime::secs(1) {
            let ret = self.stored_delta_velocity;
            self.stored_delta_velocity = Vec2::ZERO;
            ret
        } else {
            Vec2::ZERO
        }
    }

    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.angle)
    }

    pub fn target_pointing(&self) -> Option<Vec2> {
        if let VehicleController::Attitude(ta) = self.ctrl {
            Some(rotate(Vec2::X, ta))
        } else {
            None
        }
    }

    pub fn angular_velocity(&self) -> f32 {
        self.angular_velocity
    }

    pub fn angle(&self) -> f32 {
        self.angle
    }

    pub fn turn(&mut self, da: f32) {
        if let VehicleController::Attitude(ta) = &mut self.ctrl {
            *ta += da;
        }
    }

    pub fn thrusters(&self) -> impl Iterator<Item = &Thruster> + use<'_> {
        self.thrusters.iter()
    }

    pub fn thrusters_mut(&mut self) -> impl Iterator<Item = &mut Thruster> + use<'_> {
        self.thrusters.iter_mut()
    }

    pub fn bounding_radius(&self) -> f32 {
        self.bounding_radius
    }
}
