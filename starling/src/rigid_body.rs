use crate::aabb::{Polygon, AABB, OBB};
use crate::math::{linspace, rand, randint, randvec, rotate, Vec2, PI};
use crate::pv::PV;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct RigidBody {
    pub pv: PV,
    pub angle: f32,
    pub angular_rate: f32,
    pub body: Vec<AABB>,
}

pub fn asteroid_body() -> Vec<Polygon> {
    let r_avg = 30.0;
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

pub fn satellite_body() -> Vec<Polygon> {
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

impl RigidBody {
    pub fn new(pv: impl Into<PV>, body: Vec<AABB>) -> Self {
        RigidBody {
            pv: pv.into(),
            angle: 0.0,
            angular_rate: 0.0,
            body,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.pv.pos += self.pv.vel.clone() * dt;
        self.angle += self.angular_rate * dt;
        self.pv.vel *= (-dt / 8.0).exp();
        self.angular_rate *= (-dt / 4.0).exp();
    }

    pub fn mass(&self) -> f32 {
        self.body.len() as f32
    }

    // fn vel(&self, pos: Vec2) -> Vec2 {
    //     let ang = Vec3::new(0.0, 0.0, self.angular_rate);
    //     ang.cross((pos - self.pv.pos).extend(0.0)).xy() + self.pv.vel
    // }

    pub fn moi(&self) -> f32 {
        30000.0 * self.body.len() as f32
    }

    pub fn body(&self) -> Vec<OBB> {
        self.body
            .iter()
            .map(|e| e.rotate_about(Vec2::ZERO, self.angle).offset(self.pv.pos))
            .collect()
    }

    pub fn aabb(&self) -> AABB {
        let mut aabb = AABB::new(self.pv.pos, Vec2::new(5.0, 5.0));
        self.body()
            .iter()
            .map(|b| b.corners().into_iter())
            .flatten()
            .for_each(|c| {
                aabb.include(&c);
            });
        aabb
    }
}
