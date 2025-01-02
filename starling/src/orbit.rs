use crate::core::PV;
use bevy::math::Vec2;
use std::time::Duration;

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

pub fn hyperbolic_range_ta(ecc: f32) -> f32 {
    (-1.0 / ecc).acos()
}

// https://www.bogan.ca/orbits/kepler/orbteqtn.html
// https://space.stackexchange.com/questions/27602/what-is-hyperbolic-eccentric-anomaly-f
// https://orbital-mechanics.space/time-since-periapsis-and-keplers-equation/universal-variables.html

#[derive(Debug, Clone, Copy)]
pub enum Anomaly {
    Elliptical(f32),
    Parabolic(f32),
    Hyperbolic(f32),
}

pub fn true_to_eccentric(true_anomaly: Anomaly, ecc: f32) -> Anomaly {
    match true_anomaly {
        Anomaly::Elliptical(v) => Anomaly::Elliptical(f32::atan2(
            v.sin() * (1.0 - ecc.powi(2)).sqrt(),
            v.cos() + ecc,
        )),
        Anomaly::Hyperbolic(v) => Anomaly::Hyperbolic(f32::atan2(
            f32::sin(v) * (1.0 - ecc.powi(2)).sqrt(),
            f32::cos(v) + ecc,
        )),
        Anomaly::Parabolic(v) => Anomaly::Parabolic((v / 2.0).tan()),
    }
}

pub const GRAVITATIONAL_CONSTANT: f32 = 12000.0;

#[derive(Debug, Clone, Copy)]
pub struct Body {
    pub radius: f32,
    pub mass: f32,
    pub soi: f32,
}

impl Body {
    pub const fn new(radius: f32, mass: f32, soi: f32) -> Self {
        Body { radius, mass, soi }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Orbit {
    pub eccentricity: f32,
    pub semi_major_axis: f32,
    pub arg_periapsis: f32,
    pub retrograde: bool,
    pub primary_mass: f32,
    pub true_anomaly_at_epoch: f32,
}

impl Orbit {
    pub fn is_nan(&self) -> bool {
        self.eccentricity.is_nan() || self.semi_major_axis.is_nan() || self.arg_periapsis.is_nan()
    }

    pub fn from_pv(r: impl Into<Vec2>, v: impl Into<Vec2>, mass: f32) -> Self {
        let mu = mass * GRAVITATIONAL_CONSTANT;
        let r3 = r.into().extend(0.0);
        let v3 = v.into().extend(0.0);
        let h = r3.cross(v3);
        let e = v3.cross(h) / mu - r3 / r3.length();
        let arg_periapsis: f32 = f32::atan2(e.y, e.x);
        let semi_major_axis: f32 = h.length_squared() / (mu * (1.0 - e.length_squared()));
        let mut true_anomaly = f32::acos(e.dot(r3) / (e.length() * r3.length()));
        if r3.dot(v3) < 0.0 {
            true_anomaly = 2.0 * std::f32::consts::PI - true_anomaly;
        }

        Orbit {
            eccentricity: e.length(),
            semi_major_axis,
            arg_periapsis,
            retrograde: h.z < 0.0,
            primary_mass: mass,
            true_anomaly_at_epoch: true_anomaly,
        }
    }

    pub const fn circular(radius: f32, ta: f32, mass: f32) -> Self {
        Orbit {
            eccentricity: 0.0,
            semi_major_axis: radius,
            arg_periapsis: 0.0,
            retrograde: false,
            primary_mass: mass,
            true_anomaly_at_epoch: ta,
        }
    }

    pub fn prograde_at(&self, true_anomaly: f32) -> Vec2 {
        let fpa = self.flight_path_angle_at(true_anomaly);
        Vec2::from_angle(fpa).rotate(self.tangent_at(true_anomaly))
    }

    pub fn flight_path_angle_at(&self, true_anomaly: f32) -> f32 {
        -(self.eccentricity * true_anomaly.sin())
            .atan2(1.0 + self.eccentricity * true_anomaly.cos())
    }

    pub fn tangent_at(&self, true_anomaly: f32) -> Vec2 {
        let n = self.normal_at(true_anomaly);
        let angle = match self.retrograde {
            true => -std::f32::consts::PI / 2.0,
            false => std::f32::consts::PI / 2.0,
        };
        Vec2::from_angle(angle).rotate(n)
    }

    pub fn normal_at(&self, true_anomaly: f32) -> Vec2 {
        self.position_at(true_anomaly).normalize()
    }

    pub fn semi_latus_rectum(&self) -> f32 {
        if self.eccentricity == 1.0 {
            return 2.0 * self.semi_major_axis;
        }
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
    }

    pub fn angular_momentum(&self) -> f32 {
        (self.mu() * self.semi_latus_rectum()).sqrt()
    }

    pub fn radius_at(&self, true_anomaly: f32) -> f32 {
        if self.eccentricity == 1.0 {
            let h = self.angular_momentum();
            let mu = self.mu();
            return (h.powi(2) / mu) * 1.0 / (1.0 + true_anomaly.cos());
        }
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
            / (1.0 + self.eccentricity * f32::cos(true_anomaly))
    }

    pub fn period(&self) -> Option<Duration> {
        if self.eccentricity >= 1.0 {
            return None;
        }
        let t = 2.0 * std::f32::consts::PI * (self.semi_major_axis.powi(3) / (self.mu())).sqrt();
        Duration::try_from_secs_f32(t).ok()
    }

    pub fn ta_at_time(&self, mut stamp: Duration) -> f32 {
        if let Some(p) = self.period() {
            while stamp > p {
                stamp -= p;
            }
        }
        let n = self.mean_motion();
        let m0 = anomaly_t2m(self.eccentricity, self.true_anomaly_at_epoch);
        let m = stamp.as_secs_f32() * n + m0;
        anomaly_m2t(self.eccentricity, m).unwrap_or(0.0)
    }

    pub fn pv_at_time(&self, stamp: Duration) -> PV {
        let ta = self.ta_at_time(stamp);
        self.pv_at(ta)
    }

    pub fn pv_at(&self, true_anomaly: f32) -> PV {
        PV::new(
            self.position_at(true_anomaly),
            self.velocity_at(true_anomaly),
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
        let v = (self.mu() * (2.0 / r - 1.0 / self.semi_major_axis)).sqrt();
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

    pub fn periapsis_r(&self) -> f32 {
        self.radius_at(0.0)
    }

    pub fn apoapsis(&self) -> Vec2 {
        self.position_at(std::f32::consts::PI)
    }

    pub fn apoapsis_r(&self) -> f32 {
        self.radius_at(std::f32::consts::PI)
    }

    pub fn mean_motion(&self) -> f32 {
        (self.mu() / self.semi_major_axis.abs().powi(3)).sqrt()
    }

    pub fn mean_anomaly_at(&self, true_anomaly: f32) -> f32 {
        anomaly_t2m(self.eccentricity, true_anomaly)
    }

    pub fn mu(&self) -> f32 {
        GRAVITATIONAL_CONSTANT * self.primary_mass
    }
}

pub fn can_intersect(o1: &Orbit, o2: &Orbit) -> bool {
    if o1.periapsis_r() > o2.apoapsis_r() {
        false
    } else if o1.apoapsis_r() < o2.periapsis_r() {
        false
    } else {
        true
    }
}

pub fn can_intersect_soi(o1: &Orbit, o2: &Orbit, soi: f32) -> bool {
    if o1.periapsis_r() > o2.apoapsis_r() + soi {
        false
    } else if o1.apoapsis_r() + soi < o2.periapsis_r() {
        false
    } else {
        true
    }
}

pub fn will_hit_body(o: &Orbit, radius: f32) -> bool {
    o.periapsis_r() <= radius
}
