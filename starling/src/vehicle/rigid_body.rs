use crate::math::*;
use crate::nanotime::Nanotime;
use crate::pv::*;

#[derive(Default, Debug, Clone, Copy)]
pub struct BodyFrameAccel {
    pub linear: DVec2,
    pub angular: f64,
}

#[derive(Debug, Clone)]
pub struct RigidBody {
    pub pv: PV,
    pub angle: f64,
    pub angular_velocity: f64,
}

impl RigidBody {
    pub const ZERO: RigidBody = RigidBody {
        pv: PV::ZERO,
        angle: 0.0,
        angular_velocity: 0.0,
    };

    pub fn random_spin() -> Self {
        Self {
            pv: PV::ZERO,
            angle: rand(0.0, PI * 2.0) as f64,
            angular_velocity: rand(-0.3, 0.3) as f64,
        }
    }

    pub fn on_sim_tick(&mut self, a: BodyFrameAccel, gravity: DVec2, dt: Nanotime) {
        let linear = if a.linear != DVec2::ZERO {
            rotate_f64(a.linear, self.angle)
        } else {
            DVec2::ZERO
        } + gravity;

        self.angular_velocity += a.angular * dt.to_secs_f64();
        self.angular_velocity = self.angular_velocity.clamp(-2.0, 2.0);
        self.angle += self.angular_velocity * dt.to_secs_f64();
        self.angle = wrap_0_2pi_f64(self.angle);

        // TODO
        self.pv.vel += linear * dt.to_secs_f64();
        self.pv.pos += self.pv.vel * dt.to_secs_f64();
    }

    pub fn clamp_with_elevation(&mut self, elevation: f64) {
        let elev = self.pv.pos.length() as f64;
        if elev < elevation {
            self.pv.pos = self.pv.pos.normalize_or_zero() * elevation as f64;
            // self.pv.vel.y = 0.0;
        }

        // let angle = wrap_pi_npi(self.pv.pos.to_angle() as f64);

        if elev <= elevation {
            self.pv.vel *= 0.98;
            // self.angular_velocity = (angle - self.angle) * 0.1;
        }
    }
}

pub fn kinematic_apoapis(body: &RigidBody, gravity: f64) -> f64 {
    if body.pv.vel.y <= 0.0 {
        return body.pv.pos.y;
    }
    body.pv.pos.y + body.pv.vel.y.powi(2) / (2.0 * gravity.abs())
}
