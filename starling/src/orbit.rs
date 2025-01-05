use crate::core::PV;
use bevy::math::Vec2;
use chrono::TimeDelta;

pub fn as_seconds(td: TimeDelta) -> f32 {
    td.num_seconds() as f32 + td.subsec_nanos() as f32 / 1.0E9
}

pub fn hyperbolic_range_ta(ecc: f32) -> f32 {
    (-1.0 / ecc).acos()
}

// https://www.bogan.ca/orbits/kepler/orbteqtn.html
// https://space.stackexchange.com/questions/27602/what-is-hyperbolic-eccentric-anomaly-f
// https://orbital-mechanics.space/time-since-periapsis-and-keplers-equation/universal-variables.html
// http://datagenetics.com/blog/july12019/index.html

#[derive(Debug, Clone, Copy)]
pub enum Anomaly {
    Elliptical(f32),
    Parabolic(f32),
    Hyperbolic(f32),
}

fn wrap_pi_npi(x: f32) -> f32 {
    f32::atan2(x.sin(), x.cos())
}

impl Anomaly {
    pub fn with_ecc(ecc: f32, ta: f32) -> Self {
        if ecc > 1.0 {
            Anomaly::Hyperbolic(ta)
        } else if ecc == 1.0 {
            Anomaly::Parabolic(ta)
        } else {
            Anomaly::Elliptical(wrap_pi_npi(ta))
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
            Anomaly::Hyperbolic(((ecc + v.cos()) / (1. + ecc * v.cos())).acosh())
        }
        Anomaly::Parabolic(v) => Anomaly::Parabolic((v / 2.0).tan()),
    }
}

fn bhaskara_sin_approx(x: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let xp = x.abs();
    let res = 16.0 * xp / (5.0 * pi.powi(2) / (pi - x) - 4.0 * xp);
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

pub fn mean_to_eccentric(mean_anomaly: Anomaly, ecc: f32) -> Option<Anomaly> {
    match mean_anomaly {
        Anomaly::Elliptical(v) => {
            let max_error = 1E-6;
            let max_iters = 30;

            let mut e = v;

            for _ in 0..max_iters {
                e = e - (v - e + ecc * e.sin()) / (ecc * e.cos() - 1.0);
                if (v - e + ecc * e.sin()).abs() < max_error {
                    return Some(Anomaly::Elliptical(e));
                }
            }

            Some(Anomaly::Elliptical(e))
        }
        Anomaly::Hyperbolic(v) => {
            let max_error = 1E-6;
            let max_iters = 30;

            let mut e = v;

            for _ in 0..max_iters {
                e = e + (v + e - ecc * e.sinh()) / (ecc * e.cosh() - 1.0);
                if (v + e - ecc * e.sinh()).abs() < max_error {
                    return Some(Anomaly::Hyperbolic(e));
                }
            }

            Some(Anomaly::Hyperbolic(e))
        }
        Anomaly::Parabolic(_) => Some(Anomaly::Parabolic(0.0)),
    }
}

pub fn eccentric_to_true(eccentric_anomaly: Anomaly, ecc: f32) -> Anomaly {
    match eccentric_anomaly {
        Anomaly::Elliptical(v) => Anomaly::Elliptical(f32::atan2(
            f32::sin(v) * (1.0 - ecc.powi(2)).sqrt(),
            f32::cos(v) - ecc,
        )),
        Anomaly::Parabolic(_) => Anomaly::Parabolic(0.0),
        Anomaly::Hyperbolic(v) => Anomaly::Hyperbolic(
            2.0 * (((ecc + 1.0) / (ecc - 1.0)).sqrt() * (v / 2.0).tanh()).atan(),
        ),
    }
}

pub fn mean_motion(mu: f32, sma: f32) -> f32 {
    (mu / sma.abs().powi(3)).sqrt()
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
    pub time_at_periapsis: TimeDelta,
}

impl Orbit {
    pub fn is_nan(&self) -> bool {
        self.eccentricity.is_nan() || self.semi_major_axis.is_nan() || self.arg_periapsis.is_nan()
    }

    pub fn from_pv(r: impl Into<Vec2>, v: impl Into<Vec2>, mass: f32, epoch: TimeDelta) -> Self {
        let mu = mass * GRAVITATIONAL_CONSTANT;
        let r3 = r.into().extend(0.0);
        let v3 = v.into().extend(0.0);
        let h = r3.cross(v3);
        let e = v3.cross(h) / mu - r3 / r3.length();
        let arg_periapsis: f32 = f32::atan2(e.y, e.x);
        let semi_major_axis: f32 = h.length_squared() / (mu * (1.0 - e.length_squared()));
        let mut true_anomaly = f32::acos(e.dot(r3) / (e.length() * r3.length()));
        if r3.dot(v3) < 0.0 {
            true_anomaly = if e.length() < 1.0 {
                2.0 * std::f32::consts::PI - true_anomaly
            } else {
                -true_anomaly
            };
        }

        let mm = mean_motion(mu, semi_major_axis);

        let ta = Anomaly::with_ecc(e.length(), true_anomaly);
        let ea = true_to_eccentric(ta, e.length());
        let ma = eccentric_to_mean(ea, e.length());

        let dt = TimeDelta::nanoseconds((ma.as_f32() / mm * 1E9) as i64);

        // TODO this is definitely crap
        let time_at_periapsis = if e.length() > 1.0 && h.z < 0.0 {
            epoch + dt
        } else {
            epoch - dt
        };

        Orbit {
            eccentricity: e.length(),
            semi_major_axis,
            arg_periapsis,
            retrograde: h.z < 0.0,
            primary_mass: mass,
            time_at_periapsis,
        }
    }

    pub const fn circular(radius: f32, mass: f32, epoch: TimeDelta, retrograde: bool) -> Self {
        Orbit {
            eccentricity: 0.0,
            semi_major_axis: radius,
            arg_periapsis: 0.0,
            retrograde,
            primary_mass: mass,
            time_at_periapsis: epoch,
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

    pub fn period(&self) -> Option<TimeDelta> {
        if self.eccentricity >= 1.0 {
            return None;
        }
        let t = 2.0 * std::f32::consts::PI / self.mean_motion();
        Some(TimeDelta::nanoseconds((t * 1E9) as i64))
    }

    pub fn ta_at_time(&self, stamp: TimeDelta) -> Anomaly {
        let dt = stamp - self.time_at_periapsis;
        let ta = (|| {
            let n = self.mean_motion();
            let m = Anomaly::with_ecc(self.eccentricity, as_seconds(dt) * n);
            let e = mean_to_eccentric(m, self.eccentricity)?;
            Some(eccentric_to_true(e, self.eccentricity))
        })();
        Anomaly::with_ecc(self.eccentricity, ta.map_or(0.356, |a| a.as_f32()))
    }

    pub fn pv_at_time(&self, stamp: TimeDelta) -> PV {
        let ta = self.ta_at_time(stamp);
        self.pv_at(ta.as_f32())
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

    pub fn mu(&self) -> f32 {
        GRAVITATIONAL_CONSTANT * self.primary_mass
    }

    pub fn t_next_p(&self, current: TimeDelta) -> Option<TimeDelta> {
        let p = self.period()?;
        let mut t = self.time_at_periapsis;
        // while t < current {
        //     t += p;
        // }
        Some(t)
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
