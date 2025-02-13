use crate::core::Nanotime;
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
    // pub alpha: f32,
    pub retrograde: bool,
    pub body: Body,
    pub initial: PV,
    pub epoch: Nanotime,
}

impl SparseOrbit {
    pub fn from_pv(pv: impl Into<PV>, body: Body, epoch: Nanotime) -> Option<Self> {
        let pv: PV = pv.into();

        pv.filter_nan()?;

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

        let mut o = SparseOrbit {
            eccentricity: e.length(),
            semi_major_axis,
            arg_periapsis,
            retrograde: h.z < 0.0,
            body,
            initial: pv,
            epoch,
        };

        if o.pv_at_time(epoch + Nanotime::secs(1))
            .filter_nan()
            .is_none()
        {
            println!("SparseOrbit returned NaN PV: {pv:?}\n  {o:?}");
            return None;
        }

        if e.is_nan() {
            println!("Bad orbit: {pv}");
            return None;
        }

        Some(o)
    }

    pub fn circular(radius: f32, body: Body, epoch: Nanotime, retrograde: bool) -> Self {
        let p = Vec2::new(radius, 0.0);
        let v = Vec2::new(0.0, (body.mu() / radius).sqrt());
        SparseOrbit {
            eccentricity: 0.0,
            semi_major_axis: radius,
            arg_periapsis: 0.0,
            retrograde,
            body,
            initial: PV::new(p, v),
            epoch,
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

    pub fn position_at_angle(&self, angle: f32) -> Vec2 {
        let ta = angle - self.arg_periapsis;
        self.position_at(ta)
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

    pub fn pv_at_time(&self, stamp: Nanotime) -> PV {
        universal_lagrange(self.initial, stamp - self.epoch, self.body.mu())
            .map(|t| t.pv)
            .unwrap_or(PV::zero())
    }

    pub fn pv_at_time_fallible(&self, stamp: Nanotime) -> Option<PV> {
        universal_lagrange(self.initial, stamp - self.epoch, self.body.mu())
            .map(|t| t.pv)
            .ok()
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
        let dt = stamp - self.epoch;
        let n = dt.0.checked_div(p.0)?;
        Some(if dt.0 < 0 { n - 1 } else { n })
    }

    pub fn t_next_p(&self, current: Nanotime) -> Option<Nanotime> {
        None
        // if self.eccentricity >= 1.0 {
        //     return (self.time_at_periapsis >= current).then(|| self.time_at_periapsis);
        // }
        // let p = self.period()?;
        // let n = self.orbit_number(current)?;
        // Some(p * (n + 1) + self.time_at_periapsis)
    }

    pub fn t_last_p(&self, current: Nanotime) -> Option<Nanotime> {
        None
        // let p = self.period()?;
        // let n = self.orbit_number(current)?;
        // Some(p * n + self.time_at_periapsis)
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
}
