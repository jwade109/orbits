use bevy::math::Vec2;
use std::time::Duration;
use crate::core::*;

pub fn anomaly_e2m(ecc: f32, eccentric_anomaly: f32) -> f32 {
    eccentric_anomaly - ecc * f32::sin(eccentric_anomaly)
}

pub fn anomaly_m2e(ecc: f32, mean_anomaly: f32) -> Option<f32> {
    let max_error = 1E-6;
    let max_iters = 1000;

    let mut e = mean_anomaly;

    for _ in 0..max_iters {
        e = e - (mean_anomaly - e + ecc * e.sin()) / (ecc * e.cos() - 1.0);
        if (mean_anomaly - e + ecc * e.sin()).abs() < max_error {
            return Some(e);
        }
    }

    None
}

pub fn anomaly_t2e(ecc: f32, true_anomaly: f32) -> f32 {
    f32::atan2(
        f32::sin(true_anomaly) * (1.0 - ecc.powi(2)).sqrt(),
        f32::cos(true_anomaly) + ecc,
    )
}

pub fn anomaly_e2t(ecc: f32, eccentric_enomaly: f32) -> f32 {
    f32::atan2(
        f32::sin(eccentric_enomaly) * (1.0 - ecc.powi(2)).sqrt(),
        f32::cos(eccentric_enomaly) - ecc,
    )
}

pub fn anomaly_t2m(ecc: f32, true_anomaly: f32) -> f32 {
    anomaly_e2m(ecc, anomaly_t2e(ecc, true_anomaly))
}

pub fn anomaly_m2t(ecc: f32, mean_anomaly: f32) -> Option<f32> {
    anomaly_m2e(ecc, mean_anomaly).map(|e| anomaly_e2t(ecc, e))
}

pub const GRAVITATIONAL_CONSTANT: f32 = 12000.0;

#[derive(Debug, Clone, Copy)]
pub struct Body {
    pub radius: f32,
    pub mass: f32,
    pub soi: f32,
}

impl Body {
    pub fn new(radius: f32, mass: f32, soi: f32) -> Self {
        Body { radius, mass, soi }
    }

    pub fn mu(&self) -> f32 {
        self.mass * GRAVITATIONAL_CONSTANT
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Orbit {
    pub eccentricity: f32,
    pub semi_major_axis: f32,
    pub arg_periapsis: f32,
    pub true_anomaly: f32,
    pub retrograde: bool,
    pub body: Body,
}

impl Orbit {
    pub fn is_nan(&self) -> bool {
        self.eccentricity.is_nan()
            || self.semi_major_axis.is_nan()
            || self.arg_periapsis.is_nan()
            || self.true_anomaly.is_nan()
    }

    pub fn from_pv(r: impl Into<Vec2>, v: impl Into<Vec2>, body: Body) -> Self {
        let r3 = r.into().extend(0.0);
        let v3 = v.into().extend(0.0);
        let h = r3.cross(v3);
        let e = v3.cross(h) / body.mu() - r3 / r3.length();
        let arg_periapsis: f32 = f32::atan2(e.y, e.x);
        let semi_major_axis: f32 = h.length_squared() / (body.mu() * (1.0 - e.length_squared()));
        let mut true_anomaly = f32::acos(e.dot(r3) / (e.length() * r3.length()));
        if r3.dot(v3) < 0.0 {
            true_anomaly = 2.0 * std::f32::consts::PI - true_anomaly;
        }

        Orbit {
            eccentricity: e.length(),
            semi_major_axis,
            arg_periapsis,
            true_anomaly,
            retrograde: h.z < 0.0,
            body,
        }
    }

    pub fn circular(radius: f32, ta: f32, body: Body) -> Self {
        Orbit {
            eccentricity: 0.0,
            semi_major_axis: radius,
            arg_periapsis: 0.0,
            true_anomaly: ta,
            retrograde: false,
            body,
        }
    }

    pub fn prograde(&self) -> Vec2 {
        self.prograde_at(self.true_anomaly)
    }

    pub fn prograde_at(&self, true_anomaly: f32) -> Vec2 {
        let fpa = self.flight_path_angle_at(true_anomaly);
        Vec2::from_angle(fpa).rotate(self.tangent_at(true_anomaly))
    }

    pub fn flight_path_angle(&self) -> f32 {
        self.flight_path_angle_at(self.true_anomaly)
    }

    pub fn flight_path_angle_at(&self, true_anomaly: f32) -> f32 {
        -(self.eccentricity * true_anomaly.sin())
            .atan2(1.0 + self.eccentricity * true_anomaly.cos())
    }

    pub fn tangent(&self) -> Vec2 {
        self.tangent_at(self.true_anomaly)
    }

    pub fn tangent_at(&self, true_anomaly: f32) -> Vec2 {
        let n = self.normal_at(true_anomaly);
        let angle = match self.retrograde {
            true => -std::f32::consts::PI / 2.0,
            false => std::f32::consts::PI / 2.0,
        };
        Vec2::from_angle(angle).rotate(n)
    }

    pub fn normal(&self) -> Vec2 {
        self.normal_at(self.true_anomaly)
    }

    pub fn normal_at(&self, true_anomaly: f32) -> Vec2 {
        self.position_at(true_anomaly).normalize()
    }

    pub fn semi_latus_rectum(&self) -> f32 {
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
    }

    pub fn angular_momentum(&self) -> f32 {
        (self.body.mu() * self.semi_latus_rectum()).sqrt()
    }

    pub fn radius_at(&self, true_anomaly: f32) -> f32 {
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
            / (1.0 + self.eccentricity * f32::cos(true_anomaly))
    }

    pub fn period(&self) -> Duration {
        let t =
            2.0 * std::f32::consts::PI * (self.semi_major_axis.powi(3) / (self.body.mu())).sqrt();
        Duration::from_secs_f32(t)
    }

    pub fn pv(&self) -> PV {
        PV::new(
            self.position_at(self.true_anomaly),
            self.velocity_at(self.true_anomaly),
        )
    }

    pub fn position_at(&self, true_anomaly: f32) -> Vec2 {
        let r = self.radius_at(true_anomaly);
        let angle = match self.retrograde {
            false => true_anomaly,
            true => -true_anomaly,
        };
        Vec2::from_angle(angle + self.arg_periapsis) * r
    }

    pub fn velocity_at(&self, true_anomaly: f32) -> Vec2 {
        let r = self.radius_at(true_anomaly);
        let v = (self.body.mu() * (2.0 / r - 1.0 / self.semi_major_axis)).sqrt();
        let h = self.angular_momentum();
        let cosfpa = h / (r * v);
        let sinfpa = cosfpa * self.eccentricity * true_anomaly.sin()
            / (1.0 + self.eccentricity * true_anomaly.cos());
        let n = self.normal_at(true_anomaly);
        let t = self.tangent_at(true_anomaly);
        v * (t * cosfpa + n * sinfpa)
    }

    pub fn periapsis(&self) -> Vec2 {
        self.position_at(0.0)
    }

    pub fn apoapsis(&self) -> Vec2 {
        self.position_at(std::f32::consts::PI)
    }

    pub fn mean_motion(&self) -> f32 {
        (self.body.mu() / self.semi_major_axis.abs().powi(3)).sqrt()
    }

    pub fn mean_anomaly(&self) -> f32 {
        anomaly_t2m(self.eccentricity, self.true_anomaly)
    }
}

pub fn gravity_accel(body: Body, body_center: Vec2, sample: Vec2) -> Vec2 {
    let r: Vec2 = body_center - sample;
    let rsq = r.length_squared().clamp(body.radius.powi(2), std::f32::MAX);
    let a = GRAVITATIONAL_CONSTANT * body.mass / rsq;
    a * r.normalize()
}
