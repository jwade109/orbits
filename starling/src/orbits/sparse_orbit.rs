use crate::core::{rotate, Nanotime};
use crate::orbits::universal::*;
use crate::pv::PV;
use glam::f32::Vec2;

pub const PI: f32 = std::f32::consts::PI;

pub fn hyperbolic_range_ta(ecc: f32) -> f32 {
    (-1.0 / ecc).acos()
}

pub fn wrap_pi_npi(x: f32) -> f32 {
    f32::atan2(x.sin(), x.cos())
}

#[derive(Debug, Clone, Copy)]
pub enum Anomaly {
    Elliptical(f32),
    Parabolic(f32),
    Hyperbolic(f32),
}

impl Anomaly {
    pub fn with_ecc(ecc: f32, anomaly: f32) -> Self {
        if ecc > 1.0 {
            Anomaly::Hyperbolic(anomaly)
        } else if ecc == 1.0 {
            Anomaly::Parabolic(anomaly)
        } else {
            Anomaly::Elliptical(wrap_pi_npi(anomaly))
        }
    }

    pub fn as_f32(&self) -> f32 {
        match self {
            Anomaly::Elliptical(v) => *v,
            Anomaly::Parabolic(v) => *v,
            Anomaly::Hyperbolic(v) => *v,
        }
    }
}

pub fn true_to_eccentric(true_anomaly: Anomaly, ecc: f32) -> Anomaly {
    match true_anomaly {
        Anomaly::Elliptical(v) => Anomaly::Elliptical({
            let term = f32::sqrt((1. - ecc) / (1. + ecc)) * f32::tan(0.5 * v);
            2.0 * term.atan()
        }),
        Anomaly::Hyperbolic(v) => {
            let x = ((ecc + v.cos()) / (1. + ecc * v.cos())).acosh();
            Anomaly::Hyperbolic(x.abs() * v.signum())
        }
        Anomaly::Parabolic(v) => Anomaly::Parabolic((v / 2.0).tan()),
    }
}

pub fn bhaskara_sin_approx(x: f32) -> f32 {
    let xp = x.abs();
    let res = 16.0 * xp * (PI - xp) / (5.0 * PI.powi(2) - 4.0 * xp * (PI - xp));
    if x > 0.0 {
        res
    } else {
        -res
    }
}

pub fn eccentric_to_mean(eccentric_anomaly: Anomaly, ecc: f32) -> Anomaly {
    match eccentric_anomaly {
        Anomaly::Elliptical(v) => Anomaly::Elliptical(v - ecc * v.sin()),
        // Anomaly::Elliptical(v) => {
        //     Anomaly::Elliptical(v - ecc * bhaskara_sin_approx(v as f64) as f32)
        // }
        Anomaly::Hyperbolic(v) => Anomaly::Hyperbolic(ecc * v.sinh() - v),
        Anomaly::Parabolic(v) => Anomaly::Parabolic(v + v.powi(3) / 3.0),
    }
}

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

    pub fn mu(&self) -> f32 {
        self.mass * GRAVITATIONAL_CONSTANT
    }
}

const GRAVITATIONAL_CONSTANT: f32 = 12000.0;

#[derive(Debug, Clone, Copy)]
pub struct SparseOrbit {
    pub eccentricity: f32,
    pub semi_major_axis: f32,
    pub arg_periapsis: f32,
    pub retrograde: bool,
    pub body: Body,
    pub initial: PV,
    pub epoch: Nanotime,
    pub time_at_periapsis: Option<Nanotime>,
}

impl SparseOrbit {
    pub fn from_pv(pv: impl Into<PV>, body: Body, epoch: Nanotime) -> Option<Self> {
        let pv: PV = pv.into();

        pv.filter_numerr()?;

        let r3 = pv.pos.extend(0.0);
        let v3 = pv.vel.extend(0.0);
        let h = r3.cross(v3);
        let e = v3.cross(h) / body.mu() - r3 / r3.length();
        let arg_periapsis: f32 = f32::atan2(e.y, e.x);
        let semi_major_axis: f32 = h.length_squared() / (body.mu() * (1.0 - e.length_squared()));

        let mut true_anomaly = f32::acos(e.dot(r3) / (e.length() * r3.length()));
        if r3.dot(v3) < 0.0 {
            true_anomaly = if e.length() < 1.0 {
                2.0 * PI - true_anomaly
            } else {
                -true_anomaly
            };
        }

        let true_anomaly = Anomaly::with_ecc(e.length(), true_anomaly);

        let time_at_periapsis = {
            if e.length() > 0.95 {
                None
            } else {
                let eccentric_anomaly = true_to_eccentric(true_anomaly, e.length());
                let mean_anomaly = eccentric_to_mean(eccentric_anomaly, e.length());
                let mean_motion = (body.mu() / semi_major_axis.abs().powi(3)).sqrt();
                Some(epoch - Nanotime::secs_f32(mean_anomaly.as_f32() / mean_motion))
            }
        };

        let o = SparseOrbit {
            eccentricity: e.length(),
            semi_major_axis,
            arg_periapsis,
            retrograde: h.z < 0.0,
            body,
            initial: pv,
            epoch,
            time_at_periapsis,
        };

        if o.pv_at_time_fallible(epoch + Nanotime::secs(1)).is_none() {
            println!("SparseOrbit returned bad PV: {pv:?}\n  {o:?}");
            return None;
        }

        if e.is_nan() {
            println!("Bad orbit: {pv}");
            return None;
        }

        Some(o)
    }

    pub fn circular(radius: f32, body: Body, epoch: Nanotime, retrograde: bool, ta: f32) -> Self {
        let p = Vec2::new(radius, 0.0);
        let v = Vec2::new(0.0, (body.mu() / radius).sqrt());
        let p = rotate(p, ta);
        let v = rotate(v, ta);
        let mul = if retrograde { -1.0 } else { 1.0 };
        SparseOrbit::from_pv((p, mul * v), body, epoch).unwrap()
    }

    pub fn is_suborbital(&self) -> bool {
        self.periapsis_r() < self.body.radius
    }

    pub fn will_escape(&self) -> bool {
        match self.class() {
            OrbitClass::Parabolic | OrbitClass::Hyperbolic => true,
            _ => self.apoapsis_r() > self.body.soi,
        }
    }

    pub fn class(&self) -> OrbitClass {
        if self.eccentricity == 0.0 {
            OrbitClass::Circular
        } else if self.eccentricity < 0.2 {
            OrbitClass::NearCircular
        } else if self.eccentricity < 0.9 {
            OrbitClass::Elliptical
        } else if self.eccentricity < 1.0 {
            if self.eccentricity > 0.97 && self.is_suborbital() {
                OrbitClass::VeryThin
            } else {
                OrbitClass::HighlyElliptical
            }
        } else if self.eccentricity == 1.0 {
            OrbitClass::Parabolic
        } else {
            OrbitClass::Hyperbolic
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
            true => -PI / 2.0,
            false => PI / 2.0,
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

    pub fn semi_minor_axis(&self) -> f32 {
        (self.semi_major_axis.abs() * self.semi_latus_rectum().abs()).sqrt()
    }

    pub fn angular_momentum(&self) -> f32 {
        (self.body.mu() * self.semi_latus_rectum()).sqrt()
    }

    pub fn radius_at_angle(&self, angle: f32) -> f32 {
        let ta = angle - self.arg_periapsis;
        self.radius_at(ta)
    }

    pub fn pv_at_angle(&self, angle: f32) -> PV {
        let ta = if self.retrograde {
            -angle + self.arg_periapsis
        } else {
            angle - self.arg_periapsis
        };
        let pos = self.position_at(ta);
        let vel = self.velocity_at(ta);
        PV::new(pos, vel)
    }

    pub fn radius_at(&self, true_anomaly: f32) -> f32 {
        if self.eccentricity == 1.0 {
            let h = self.angular_momentum();
            let mu = self.body.mu();
            return (h.powi(2) / mu) * 1.0 / (1.0 + true_anomaly.cos());
        }
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
            / (1.0 + self.eccentricity * f32::cos(true_anomaly))
    }

    pub fn period(&self) -> Option<Nanotime> {
        if self.eccentricity >= 1.0 {
            return None;
        }
        let t = 2.0 * PI / self.mean_motion();
        Some(Nanotime((t * 1E9) as i64))
    }

    pub fn period_or(&self, fallback: Nanotime) -> Nanotime {
        self.period().unwrap_or(fallback)
    }

    pub fn pv_at_time(&self, stamp: Nanotime) -> PV {
        self.pv_at_time_fallible(stamp).unwrap_or_else(|| {
            dbg!(self);
            panic!()
        })
    }

    pub fn pv_at_time_fallible(&self, stamp: Nanotime) -> Option<PV> {
        let advance = match (self.orbit_number(stamp), self.period()) {
            (Some(n), Some(p)) => p * n,
            _ => Nanotime(0),
        };
        let tof = stamp - (self.epoch + advance);
        universal_lagrange(self.initial, tof, self.body.mu())
            .1
            .map(|t| t.pv.filter_numerr())
            .flatten()
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

    pub fn periapsis_r(&self) -> f32 {
        self.radius_at(0.0)
    }

    pub fn apoapsis(&self) -> Vec2 {
        self.position_at(PI)
    }

    pub fn apoapsis_r(&self) -> f32 {
        self.radius_at(PI)
    }

    pub fn mean_motion(&self) -> f32 {
        (self.body.mu() / self.semi_major_axis.abs().powi(3)).sqrt()
    }

    pub fn orbit_number(&self, stamp: Nanotime) -> Option<i64> {
        let p = self.period()?;
        let dt = stamp - self.time_at_periapsis?;
        let n = dt.0.checked_div(p.0)?;
        Some(if dt.0 < 0 { n - 1 } else { n })
    }

    pub fn t_next_p(&self, current: Nanotime) -> Option<Nanotime> {
        let tp = self.time_at_periapsis?;
        if self.eccentricity >= 1.0 {
            return (tp >= current).then(|| tp);
        }
        let p = self.period()?;
        let n = self.orbit_number(current)?;
        Some(p * (n + 1) + tp)
    }

    pub fn t_last_p(&self, current: Nanotime) -> Option<Nanotime> {
        let p = self.period()?;
        let n = self.orbit_number(current)?;
        Some(p * n + self.time_at_periapsis?)
    }

    pub fn asymptotes(&self) -> Option<(Vec2, Vec2)> {
        if self.eccentricity < 1.0 {
            return None;
        }
        let u = self.periapsis().normalize();
        let b = self.semi_minor_axis();

        let ua = Vec2::new(self.semi_major_axis, b);
        let ub = Vec2::new(self.semi_major_axis, -b);

        Some((u.rotate(ua), u.rotate(ub)))
    }

    pub fn nearest_along_track(&self, pos: Vec2) -> (PV, f32) {
        let angle = -pos.angle_to(Vec2::X);
        let p = self.pv_at_angle(angle);
        let d = p.pos.distance(pos);
        if p.pos.length() > pos.length() {
            (p, -d)
        } else {
            (p, d)
        }
    }

    pub fn nearest(&self, pos: Vec2) -> (PV, f32) {
        let (mut ret, mut dist) = self.nearest_along_track(pos);
        let sign = dist.signum();
        let mut test_pos = pos;
        for _ in 0..4 {
            let (pv, d) = self.nearest_along_track(test_pos);
            let u = match pv.vel.try_normalize() {
                Some(u) => u,
                None => return (pv, d),
            };
            let diff = pos - pv.pos;
            let mag = diff.dot(u);
            test_pos = pv.pos + mag * u;
            dist = d;
            ret = pv;
        }

        (ret, dist.abs() * sign)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OrbitClass {
    Circular,
    NearCircular,
    Elliptical,
    HighlyElliptical,
    Parabolic,
    Hyperbolic,
    VeryThin,
}
