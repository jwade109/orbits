use crate::math::*;
use crate::nanotime::Nanotime;
use crate::pv::*;

#[derive(Default, Debug, Clone, Copy)]
pub struct BodyFrameAccel {
    pub linear: Vec2,
    pub angular: f32,
}

#[derive(Debug, Clone)]
pub struct RigidBody {
    pub pv: PV,
    pub angle: f32,
    pub angular_velocity: f32,
}

impl RigidBody {
    pub const ZERO: RigidBody = RigidBody {
        pv: PV::ZERO,
        angle: 0.0,
        angular_velocity: 0.0,
    };

    pub fn on_sim_tick(&mut self, a: BodyFrameAccel, gravity: Vec2, dt: Nanotime) {
        let linear = if a.linear != Vec2::ZERO {
            rotate(a.linear, self.angle)
        } else {
            Vec2::ZERO
        } + gravity;

        self.angular_velocity += a.angular * dt.to_secs();
        self.angular_velocity = self.angular_velocity.clamp(-2.0, 2.0);
        self.angle += self.angular_velocity * dt.to_secs();
        self.angle = wrap_0_2pi(self.angle);

        // TODO
        self.pv.vel += linear.as_dvec2() * dt.to_secs_f64();
        self.pv.pos += self.pv.vel * dt.to_secs_f64();
    }

    pub fn on_the_floor(&mut self, elevation: f32) {
        if self.pv.pos.y < elevation as f64 {
            self.pv.pos.y = elevation as f64;
            self.pv.vel.y = 0.0;
        }

        if self.pv.pos.y == elevation as f64 {
            self.pv.vel.x = 0.0;
            self.angular_velocity = (PI / 2.0 - self.angle) * 0.1;
        }
    }
}

pub fn kinematic_apoapis(body: &RigidBody, gravity: f64) -> f64 {
    if body.pv.vel.y <= 0.0 {
        return body.pv.pos.y;
    }
    body.pv.pos.y + body.pv.vel.y.powi(2) / (2.0 * gravity.abs())
}
