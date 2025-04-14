use crate::aabb::Polygon;
use crate::math::{rand, rotate, Vec2, PI};
use crate::nanotime::Nanotime;
use crate::rigid_body::{asteroid_body, satellite_body};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Thruster {
    pub pos: Vec2,
    pub angle: f32,
    pub length: f32,
}

impl Thruster {
    pub fn new(pos: Vec2, angle: f32, length: f32) -> Self {
        Self { pos, angle, length }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Vehicle {
    stamp: Nanotime,
    angle: f32,
    angular_velocity: f32,
    angular_acceleration: f32,
    body: Vec<Polygon>,
    thrusters: Vec<Thruster>,
}

impl Vehicle {
    pub fn new(stamp: Nanotime) -> Self {
        let is_asteroid = rand(0.0, 1.0) < 0.4;
        let (body, avel) = if is_asteroid {
            (asteroid_body(), rand(-0.3, 0.3))
        } else {
            (satellite_body(), 0.0)
        };

        let thrusters = if is_asteroid {
            vec![]
        } else {
            vec![Thruster::new(Vec2::X * -0.8, 0.0, 1.0)]
        };

        Self {
            body,
            angle: rand(0.0, PI * 2.0),
            angular_velocity: avel,
            angular_acceleration: 0.0,
            stamp,
            thrusters,
        }
    }

    pub fn is_controllable(&self) -> bool {
        !self.thrusters.is_empty()
    }

    pub fn step(&mut self, stamp: Nanotime) {
        let dt = (stamp - self.stamp).to_secs();
        self.angle += self.angular_velocity * dt;
        self.angular_velocity += self.angular_acceleration * dt;
        self.angular_acceleration = 0.0;
        if self.is_controllable() {
            self.angular_velocity *= (-dt / 5.0).exp();
        }
        self.stamp = stamp;
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

    pub fn torque(&mut self, torque: f32) {
        self.angular_acceleration = torque
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
