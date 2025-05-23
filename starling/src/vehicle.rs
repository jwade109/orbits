use crate::aabb::{Polygon, AABB};
use crate::inventory::{Inventory, InventoryItem};
use crate::math::{
    cross2d, get_random_name, linspace, rand, randint, randvec, rotate, IVec2, Vec2, PI,
};
use crate::nanotime::Nanotime;
use crate::orbits::{wrap_0_2pi, wrap_pi_npi};
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Sequence, Hash)]
pub enum PartLayer {
    Internal,
    Structural,
    Exterior,
}

/// dimensions in meters
#[derive(Debug, Clone, Copy)]
pub struct PartProto {
    pub width: u32,
    pub height: u32,
    pub layer: PartLayer,
    pub path: &'static str,
}

impl PartProto {
    pub const fn new(width: u32, height: u32, layer: PartLayer, path: &'static str) -> Self {
        Self {
            width,
            height,
            layer,
            path,
        }
    }

    pub fn to_z_index(&self) -> f32 {
        match self.layer {
            PartLayer::Internal => 10.0,
            PartLayer::Structural => 11.0,
            PartLayer::Exterior => 12.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Thruster {
    pub pos: Vec2,
    pub angle: f32,
    pub length: f32,
    pub is_active: bool,
    pub is_rcs: bool,
}

impl Thruster {
    pub fn main(pos: Vec2, angle: f32, length: f32) -> Self {
        Self {
            pos,
            angle,
            length,
            is_active: false,
            is_rcs: false,
        }
    }

    pub fn rcs(pos: Vec2, angle: f32) -> Self {
        Self {
            pos,
            angle,
            length: 0.14,
            is_active: false,
            is_rcs: true,
        }
    }

    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.angle)
    }
}

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Vehicle {
    name: String,
    stamp: Nanotime,
    angle: f32,
    target_angle: f32,
    angular_velocity: f32,
    body: Vec<Polygon>,
    thrusters: Vec<Thruster>,
    pub inventory: Inventory,
    pub max_fuel_mass: f32,
    pub dry_mass: f32,
    pub exhaust_velocity: f32,
    pub parts: Vec<(IVec2, Rotation, String)>,
}

fn rcs_block(pos: Vec2, angle: f32) -> Vec<Thruster> {
    vec![
        Thruster::rcs(pos, angle - 0.4 * PI),
        Thruster::rcs(pos, angle + 0.0 * PI),
        Thruster::rcs(pos, angle + 0.4 * PI),
    ]
}

fn random_sat_inventory() -> Inventory {
    use InventoryItem::*;
    let mut inv = Inventory::new();
    inv.add(Copper, randint(2000, 5000) as u64);
    inv.add(Silicon, randint(40, 400) as u64);
    inv.add(LiquidFuel, randint(500, 800) as u64 * 1000);
    inv
}

fn rocket_equation(ve: f32, m0: f32, m1: f32) -> f32 {
    ve * (m0 / m1).ln()
}

fn mass_after_maneuver(ve: f32, m0: f32, dv: f32) -> f32 {
    m0 / (dv / ve).exp()
}

impl Vehicle {
    pub fn random(stamp: Nanotime) -> Self {
        if rand(0.0, 1.0) < 0.5 {
            Self::asteroid(stamp)
        } else {
            Self::satellite(stamp)
        }
    }

    pub fn asteroid(stamp: Nanotime) -> Self {
        Self {
            max_fuel_mass: 0.0,
            dry_mass: 300.0,
            exhaust_velocity: 5000.0,
            name: get_random_name(),
            stamp,
            angle: rand(0.0, 2.0 * PI),
            target_angle: 0.0,
            angular_velocity: rand(-0.3, 0.3),
            body: asteroid_body(),
            thrusters: Vec::new(),
            inventory: Inventory::new(),
            parts: vec![
                (IVec2::ZERO, Rotation::East, "tank22".into()),
                (IVec2::X * 20, Rotation::North, "frame3".into()),
            ],
        }
    }

    pub fn satellite(stamp: Nanotime) -> Self {
        Self {
            max_fuel_mass: 800.0,
            dry_mass: 300.0,
            exhaust_velocity: 5000.0,
            name: get_random_name(),
            stamp,
            angle: rand(0.0, 2.0 * PI),
            target_angle: rand(0.0, 2.0 * PI),
            angular_velocity: rand(-0.3, 0.3),
            body: satellite_body(),
            thrusters: satellite_thrusters(),
            inventory: random_sat_inventory(),
            parts: vec![(IVec2::ZERO, Rotation::East, "frame".into())],
        }
    }

    pub fn space_station(stamp: Nanotime) -> Self {
        Self {
            max_fuel_mass: 800.0,
            dry_mass: 300.0,
            exhaust_velocity: 5000.0,
            name: get_random_name(),
            stamp,
            angle: 0.0,
            target_angle: 0.0,
            angular_velocity: 0.0,
            body: space_station_body(),
            thrusters: rcs_block(Vec2::X, 0.0),
            inventory: random_sat_inventory(),
            parts: vec![(IVec2::ZERO, Rotation::East, "frame2".into())],
        }
    }

    pub fn is_controllable(&self) -> bool {
        !self.thrusters.is_empty()
    }

    pub fn fuel_mass(&self) -> f32 {
        self.inventory.count(InventoryItem::LiquidFuel) as f32 / 1000.0
    }

    pub fn mass(&self) -> f32 {
        self.dry_mass + self.fuel_mass()
    }

    pub fn low_fuel(&self) -> bool {
        self.is_controllable() && self.remaining_dv() < 10.0
    }

    pub fn add_fuel(&mut self, kg: u64) {
        self.inventory.add(InventoryItem::LiquidFuel, kg * 1000);
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
        let dt = (stamp - self.stamp).to_secs().clamp(0.01, 0.5);

        if self.is_controllable() {
            let kp = 100.0;
            let kd = 50.0;

            let error = wrap_pi_npi(self.target_angle - self.angle);

            let angular_acceleration = kp * error - kd * self.angular_velocity;

            for t in &mut self.thrusters {
                if !t.is_rcs {
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

    pub fn main(&mut self, is_high: bool) {
        for t in &mut self.thrusters {
            if !t.is_rcs {
                t.is_active = is_high;
            }
        }
    }

    pub fn body(&self) -> impl Iterator<Item = Polygon> + use<'_> {
        self.body
            .iter()
            .map(move |a| a.rotate_about(Vec2::ZERO, self.angle))
    }

    pub fn thrusters(&self) -> impl Iterator<Item = &Thruster> + use<'_> {
        self.thrusters.iter()
    }

    pub fn bounding_radius(&self) -> f32 {
        let mut max: f32 = 0.0;
        self.body
            .iter()
            .flat_map(|p| p.iter())
            .for_each(|p| max = max.max(p.length()));
        max
    }
}

fn asteroid_body() -> Vec<Polygon> {
    let r_avg = rand(7.0, 45.0);
    let potato = |p: Vec2, r: f32| -> Polygon {
        let m1 = randint(2, 5) as f32;
        let m2 = randint(3, 12) as f32;
        let m3 = randint(7, 16) as f32;
        let m4 = randint(12, 20) as f32;

        Polygon::new(
            linspace(0.0, 2.0 * PI, 40)
                .iter()
                .map(|a| {
                    let r = r
                        + ((a * m1).sin() * r * 0.3)
                        + ((a * m2).sin() * r * 0.1)
                        + ((a * m3).sin() * r * 0.02)
                        + ((a * m4).sin() * r * 0.01);
                    p + rotate(Vec2::X * r, *a)
                })
                .collect(),
        )
    };

    let mut potatoes = vec![potato(Vec2::ZERO, r_avg)];

    for _ in 0..randint(1, 4) {
        let center = randvec(r_avg * 0.1, r_avg * 0.8);
        let s = r_avg * rand(0.2, 0.6);
        potatoes.push(potato(center, s));
    }

    potatoes
}

fn satellite_body() -> Vec<Polygon> {
    vec![
        // body
        AABB::from_arbitrary((-0.9, -0.9), (0.0, 0.0)).polygon(),
        AABB::from_arbitrary((0.9, -0.9), (0.0, 0.0)).polygon(),
        AABB::from_arbitrary((0.0, 0.0), (0.9, 0.9)).polygon(),
        AABB::from_arbitrary((-0.9, 0.9), (0.0, 0.0)).polygon(),
        // panels
        AABB::from_arbitrary((0.2, -3.0), (-0.2, -0.9)).polygon(),
        AABB::from_arbitrary((0.2, 3.0), (-0.2, 0.9)).polygon(),
        // front
        AABB::from_arbitrary((0.9, -0.5), (1.5, 0.5)).polygon(),
        Polygon::circle((-0.97, 0.0), 0.2, 16),
        Polygon::circle((-0.97, -0.4), 0.2, 16),
        Polygon::circle((-0.97, 0.4), 0.2, 16),
    ]
}

fn space_station_body() -> Vec<Polygon> {
    (-2..=2)
        .map(|i| {
            let center = Vec2::new(i as f32 * 10.0, 0.0);
            let upper = Polygon::circle(center + Vec2::Y * 6.0, 2.0, 8);
            let lower = Polygon::circle(center - Vec2::Y * 6.0, 2.0, 8);
            let tube = AABB::new(center, Vec2::new(10.0, 4.0));
            [upper, lower, tube.polygon()]
        })
        .flatten()
        .collect()
}

fn satellite_thrusters() -> Vec<Thruster> {
    let mut t = vec![Thruster::main(Vec2::X * -0.8, 0.0, 1.0)];
    t.extend(rcs_block(Vec2::new(0.8, 0.9), -0.5 * PI));
    t.extend(rcs_block(Vec2::new(0.8, -0.9), 0.5 * PI));
    t.extend(rcs_block(Vec2::new(-0.8, 0.9), -0.5 * PI));
    t.extend(rcs_block(Vec2::new(-0.8, -0.9), 0.5 * PI));
    t
}

pub const TANK11: PartProto = PartProto::new(10, 10, PartLayer::Internal, "tank11");
pub const TANK21: PartProto = PartProto::new(10, 20, PartLayer::Internal, "tank21");
pub const TANK22: PartProto = PartProto::new(20, 20, PartLayer::Internal, "tank22");
pub const FRAME: PartProto = PartProto::new(10, 10, PartLayer::Structural, "frame");
pub const FRAME2: PartProto = PartProto::new(10, 10, PartLayer::Structural, "frame2");
pub const FRAME22: PartProto = PartProto::new(20, 20, PartLayer::Structural, "frame22");
pub const FRAME3: PartProto = PartProto::new(40, 10, PartLayer::Structural, "frame3");
pub const MOTOR: PartProto = PartProto::new(16, 25, PartLayer::Internal, "motor");
pub const ANTENNA: PartProto = PartProto::new(50, 27, PartLayer::Internal, "antenna");
pub const SMALL_ANTENNA: PartProto = PartProto::new(6, 20, PartLayer::Internal, "small-antenna");
pub const CARGO: PartProto = PartProto::new(30, 30, PartLayer::Internal, "cargo");
pub const BATTERY: PartProto = PartProto::new(9, 9, PartLayer::Internal, "battery");
pub const CPU: PartProto = PartProto::new(8, 9, PartLayer::Internal, "cpu");
pub const SOLARPANEL: PartProto = PartProto::new(65, 16, PartLayer::Internal, "solarpanel");
pub const GOLD: PartProto = PartProto::new(10, 10, PartLayer::Exterior, "gold");
pub const PLATE: PartProto = PartProto::new(10, 10, PartLayer::Exterior, "plate");

pub fn find_part(short_path: &str) -> Option<&PartProto> {
    ALL_PARTS.iter().cloned().find(|p| p.path == short_path)
}

pub const ALL_PARTS: [&PartProto; 16] = [
    &TANK11,
    &TANK21,
    &TANK22,
    &FRAME,
    &FRAME2,
    &FRAME22,
    &FRAME3,
    &MOTOR,
    &ANTENNA,
    &SMALL_ANTENNA,
    &CARGO,
    &BATTERY,
    &CPU,
    &SOLARPANEL,
    &GOLD,
    &PLATE,
];
