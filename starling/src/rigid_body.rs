use crate::aabb::{AABB, OBB};
use crate::math::{rotate, Vec2, PI};
use crate::pv::PV;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct RigidBody {
    pub pv: PV,
    pub angle: f32,
    pub angular_rate: f32,
    pub body: Vec<AABB>,
}

pub fn satellite_body(scale: f32) -> Vec<AABB> {
    vec![
        // body
        AABB::from_arbitrary((-100.0, -100.0), (0.0, 0.0)),
        AABB::from_arbitrary((90.0, -90.0), (0.0, 0.0)),
        AABB::from_arbitrary((0.0, 0.0), (100.0, 100.0)),
        AABB::from_arbitrary((-90.0, 90.0), (0.0, 0.0)),
        // panels
        AABB::from_arbitrary((-300.0, 20.0), (-80.0, -20.0)),
        AABB::from_arbitrary((300.0, 20.0), (80.0, -20.0)),
        // thruster
        AABB::from_arbitrary((-50.0, -90.0), (50.0, -150.0)),
    ]
    .iter()
    .map(|a| a.scale(scale))
    .map(|a| {
        let p1 = rotate(a.center + a.span / 2.0, PI / 2.0);
        let p2 = rotate(a.center - a.span / 2.0, PI / 2.0);
        AABB::from_arbitrary(p1, p2)
    })
    .collect()
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
