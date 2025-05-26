use crate::inventory::{Inventory, InventoryItem};
use crate::math::{cross2d, get_random_name, rand, randint, rotate, IVec2, Vec2, PI};
use crate::nanotime::Nanotime;
use crate::orbits::{wrap_0_2pi, wrap_pi_npi};
use crate::parts::{
    parts::{PartClass, PartProto},
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

#[derive(Debug, Clone)]
pub struct Vehicle {
    name: String,
    stamp: Nanotime,
    angle: f32,
    target_angle: f32,
    angular_velocity: f32,
    thrusters: Vec<Thruster>,
    tanks: Vec<Tank>,
    bounding_radius: f32,
    pub inventory: Inventory,
    pub max_fuel_mass: f32,
    pub dry_mass: f32,
    pub exhaust_velocity: f32,
    pub parts: Vec<(IVec2, Rotation, PartProto)>,
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

impl Vehicle {
    pub fn from_parts(stamp: Nanotime, parts: Vec<(IVec2, Rotation, PartProto)>) -> Self {
        let thrusters: Vec<Thruster> = parts
            .iter()
            .filter_map(|(pos, _, p)| {
                if let PartClass::Thruster(proto) = p.data.class {
                    Some(Thruster {
                        proto,
                        pos: pos.as_vec2(),
                        angle: 0.0,
                        is_active: false,
                    })
                } else {
                    None
                }
            })
            .collect();

        let tanks: Vec<Tank> = parts
            .iter()
            .filter_map(|(_, _, p)| {
                if let PartClass::Tank(proto) = p.data.class {
                    Some(Tank {
                        proto,
                        fuel_mass: (proto.wet_mass - p.data.mass) * 0.8,
                    })
                } else {
                    None
                }
            })
            .collect();

        let mut bounding_radius = 1.0;
        for (pos, _, part) in &parts {
            let r = part.width.max(part.height) as f32 / 2.0;
            let d = pos.as_vec2().length() + r;
            if d > bounding_radius {
                bounding_radius = d;
            }
        }

        Self {
            max_fuel_mass: 0.0,
            dry_mass: 300.0,
            exhaust_velocity: 5000.0,
            name: get_random_name(),
            stamp,
            angle: rand(0.0, 2.0 * PI),
            target_angle: rand(0.0, 2.0 * PI),
            angular_velocity: rand(-0.3, 0.3),
            tanks,
            thrusters,
            inventory: random_sat_inventory(),
            parts,
            bounding_radius,
        }
    }

    pub fn is_controllable(&self) -> bool {
        !self.thrusters.is_empty()
    }

    pub fn fuel_mass(&self) -> f32 {
        self.tanks.iter().map(|t| t.fuel_mass).sum()
    }

    pub fn mass(&self) -> f32 {
        self.dry_mass + self.fuel_mass()
    }

    pub fn low_fuel(&self) -> bool {
        self.is_controllable() && self.remaining_dv() < 10.0
    }

    pub fn add_fuel(&mut self, _kg: u64) {
        todo!()
    }

    pub fn try_impulsive_burn(&mut self, dv: Vec2) -> Option<()> {
        if dv.length() > self.remaining_dv() {
            return None;
        }

        let fuel_mass_before_maneuver = self.fuel_mass();
        let m1 = mass_after_maneuver(self.exhaust_velocity, self.mass(), dv.length());
        let fuel_mass_after_maneuver = m1 - self.dry_mass;
        let spent_fuel = fuel_mass_before_maneuver - fuel_mass_after_maneuver;

        self.inventory.take(
            InventoryItem::LiquidFuel,
            (spent_fuel * 1000.0).round() as u64,
        );

        Some(())
    }

    pub fn remaining_dv(&self) -> f32 {
        rocket_equation(self.exhaust_velocity, self.mass(), self.dry_mass)
    }

    pub fn fuel_percentage(&self) -> f32 {
        self.fuel_mass() / self.max_fuel_mass
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn step(&mut self, stamp: Nanotime) {
        let dt = (stamp - self.stamp).to_secs().clamp(0.0001, 0.03);

        if self.is_controllable() {
            let kp = 50.0;
            let kd = 20.0;

            let error = wrap_pi_npi(self.target_angle - self.angle);

            let angular_acceleration = kp * error - kd * self.angular_velocity;

            for t in &mut self.thrusters {
                if !t.proto.is_rcs {
                    continue;
                }

                let torque = cross2d(t.pos, t.pointing());
                let sign_equal = torque.signum() == angular_acceleration.signum();
                let is_thrusting = angular_acceleration.abs() > 0.01;

                t.is_active = sign_equal && is_thrusting;
            }

            self.angular_velocity += angular_acceleration * dt;
        }

        self.angle += self.angular_velocity * dt;
        self.angle = wrap_0_2pi(self.angle);
        self.target_angle = wrap_0_2pi(self.target_angle);
        self.stamp = stamp;
    }

    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.angle)
    }

    pub fn target_pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.target_angle)
    }

    pub fn angular_velocity(&self) -> f32 {
        self.angular_velocity
    }

    pub fn angle(&self) -> f32 {
        self.angle
    }

    pub fn turn(&mut self, da: f32) {
        self.target_angle += da;
    }

    pub fn thrusters(&self) -> impl Iterator<Item = &Thruster> + use<'_> {
        self.thrusters.iter()
    }

    pub fn bounding_radius(&self) -> f32 {
        self.bounding_radius
    }
}
