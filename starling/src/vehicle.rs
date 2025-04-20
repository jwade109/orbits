use crate::aabb::{Polygon, AABB};
use crate::inventory::{Inventory, InventoryItem};
use crate::math::{cross2d, get_random_name, linspace, rand, randint, randvec, rotate, Vec2, PI};
use crate::nanotime::Nanotime;
use serde::{Deserialize, Serialize};

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
            is_active: rand(0.0, 1.0) < 0.7,
            is_rcs: false,
        }
    }

    pub fn rcs(pos: Vec2, angle: f32) -> Self {
        Self {
            pos,
            angle,
            length: 0.14,
            is_active: rand(0.0, 1.0) < 0.7,
            is_rcs: true,
        }
    }

    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.angle)
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
            name: get_random_name(),
            stamp,
            angle: rand(0.0, 2.0 * PI),
            target_angle: 0.0,
            angular_velocity: rand(-0.3, 0.3),
            body: asteroid_body(),
            thrusters: Vec::new(),
            inventory: Inventory::new(),
        }
    }

    pub fn satellite(stamp: Nanotime) -> Self {
        Self {
            name: get_random_name(),
            stamp,
            angle: rand(0.0, 2.0 * PI),
            target_angle: rand(0.0, 2.0 * PI),
            angular_velocity: rand(-0.3, 0.3),
            body: satellite_body(),
            thrusters: satellite_thrusters(),
            inventory: random_sat_inventory(),
        }
    }

    pub fn space_station(stamp: Nanotime) -> Self {
        Self {
            name: get_random_name(),
            stamp,
            angle: 0.0,
            target_angle: 0.0,
            angular_velocity: 0.0,
            body: space_station_body(),
            thrusters: rcs_block(Vec2::X, 0.0),
            inventory: random_sat_inventory(),
        }
    }

    pub fn is_controllable(&self) -> bool {
        !self.thrusters.is_empty()
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn step(&mut self, stamp: Nanotime) {
        let dt = (stamp - self.stamp).to_secs().min(0.1);

        if self.is_controllable() {
            let kp = 100.0;
            let kd = 50.0;

            let angular_acceleration =
                kp * (self.target_angle - self.angle) - kd * self.angular_velocity;

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
    (-2..=2).map(|i| {
        let center = Vec2::new(i as f32 * 10.0, 0.0);
        let upper = Polygon::circle(center + Vec2::Y * 6.0, 2.0, 8);
        let lower = Polygon::circle(center - Vec2::Y * 6.0, 2.0, 8);
        let tube = AABB::new(center, Vec2::new(10.0, 4.0));
        [upper, lower, tube.polygon()]
    }).flatten().collect()
}

fn satellite_thrusters() -> Vec<Thruster> {
    let mut t = vec![Thruster::main(Vec2::X * -0.8, 0.0, 1.0)];
    t.extend(rcs_block(Vec2::new(0.8, 0.9), -0.5 * PI));
    t.extend(rcs_block(Vec2::new(0.8, -0.9), 0.5 * PI));
    t.extend(rcs_block(Vec2::new(-0.8, 0.9), -0.5 * PI));
    t.extend(rcs_block(Vec2::new(-0.8, -0.9), 0.5 * PI));
    t
}
