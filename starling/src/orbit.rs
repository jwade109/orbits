use crate::core::*;
use bevy::math::Vec2;

pub const PI: f32 = std::f32::consts::PI;

pub fn as_seconds(t: Nanotime) -> f32 {
    let ns = 1000000000;
    (t.0 / ns) as f32 + (t.0 % ns) as f32 / ns as f32
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

pub fn wrap_pi_npi(x: f32) -> f32 {
    f32::atan2(x.sin(), x.cos())
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

pub fn mean_to_eccentric(mean_anomaly: Anomaly, ecc: f32) -> Option<Anomaly> {
    match mean_anomaly {
        Anomaly::Elliptical(v) => {
            let max_error = 1E-4;
            let max_iters = 40;

            let mut e = match (v > 0.0, ecc > 0.8) {
                (true, true) => PI,
                (false, true) => -PI,
                (_, false) => v,
            };

            for _ in 0..max_iters {
                e = e - (v - e + ecc * e.sin()) / (ecc * e.cos() - 1.0);
                if (v - e + ecc * e.sin()).abs() < max_error {
                    return Some(Anomaly::Elliptical(e));
                }
            }

            None
        }
        Anomaly::Parabolic(v) | Anomaly::Hyperbolic(v) => {
            let max_error = 1E-4;
            let max_iters = 40;

            let mut e = v.abs().sqrt() * v.signum();

            for _ in 0..max_iters {
                e = e + (v + e - ecc * e.sinh()) / (ecc * e.cosh() - 1.0);
                if (v + e - ecc * e.sinh()).abs() < max_error {
                    return Some(Anomaly::Hyperbolic(e));
                }
            }

            None
        }
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
    pub time_at_periapsis: Nanotime,
}

impl Orbit {
    pub fn is_nan(&self) -> bool {
        self.eccentricity.is_nan() || self.semi_major_axis.is_nan() || self.arg_periapsis.is_nan()
    }

    pub fn from_pv(r: impl Into<Vec2>, v: impl Into<Vec2>, mass: f32, epoch: Nanotime) -> Self {
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
                2.0 * PI - true_anomaly
            } else {
                -true_anomaly
            };
        }

        let mm = mean_motion(mu, semi_major_axis);

        let ta = Anomaly::with_ecc(e.length(), true_anomaly);
        let ea = true_to_eccentric(ta, e.length());
        let ma = eccentric_to_mean(ea, e.length());

        let dt = Nanotime((ma.as_f32() / mm * 1E9) as i64);

        // TODO this is definitely crap
        let time_at_periapsis = if e.length() > 1.0 && h.z < 0.0 {
            epoch + dt
        } else {
            epoch - dt
        };

        let mut o = Orbit {
            eccentricity: e.length(),
            semi_major_axis,
            arg_periapsis,
            retrograde: h.z < 0.0,
            primary_mass: mass,
            time_at_periapsis,
        };

        // TODO mega turbo crap
        let pcalc = o.pv_at_time(epoch);
        if pcalc.pos.distance(Vec2::new(r3.x, r3.y)) > 20.0 {
            o.time_at_periapsis = if e.length() > 1.0 && h.z < 0.0 {
                epoch - dt
            } else {
                epoch + dt
            };
        }

        o
    }

    pub fn from_points(p1: Vec2, p2: Vec2, p3: Vec2, mass: f32) -> Option<Self> {
        let r1 = p1.length();
        let r2 = p2.length();
        let r3 = p3.length();
        let r = (r1 + r2 + r3) / 3.0;
        Some(Orbit::circular(r, mass, Nanotime(0), false))
    }

    pub const fn circular(radius: f32, mass: f32, epoch: Nanotime, retrograde: bool) -> Self {
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
        (self.mu() * self.semi_latus_rectum()).sqrt()
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
            let mu = self.mu();
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

    pub fn ma_at_time(&self, stamp: Nanotime) -> Anomaly {
        let dt = stamp - self.t_last_p(stamp).unwrap_or(self.time_at_periapsis);
        let n = self.mean_motion();
        Anomaly::with_ecc(self.eccentricity, as_seconds(dt) * n)
    }

    pub fn ea_at_time(&self, stamp: Nanotime) -> Anomaly {
        let m = self.ma_at_time(stamp);
        mean_to_eccentric(m, self.eccentricity).unwrap_or(Anomaly::with_ecc(self.eccentricity, 0.0))
    }

    pub fn ta_at_time(&self, stamp: Nanotime) -> Anomaly {
        let e = self.ea_at_time(stamp);
        eccentric_to_true(e, self.eccentricity)
    }

    pub fn pv_at_time(&self, stamp: Nanotime) -> PV {
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
        self.position_at(PI)
    }

    pub fn apoapsis_r(&self) -> f32 {
        self.radius_at(PI)
    }

    pub fn mean_motion(&self) -> f32 {
        (self.mu() / self.semi_major_axis.abs().powi(3)).sqrt()
    }

    pub fn mu(&self) -> f32 {
        GRAVITATIONAL_CONSTANT * self.primary_mass
    }

    pub fn normalize(&mut self, stamp: Nanotime) -> Option<()> {
        let num = self.orbit_number(stamp)?;
        let p = self.period()?;
        self.time_at_periapsis += p * num;
        Some(())
    }

    pub fn orbit_number(&self, stamp: Nanotime) -> Option<i64> {
        let p = self.period()?;
        let dt = stamp - self.time_at_periapsis;
        let n = dt.0.checked_div(p.0)?;
        Some(if dt.0 < 0 { n - 1 } else { n })
    }

    pub fn t_next_p(&self, current: Nanotime) -> Option<Nanotime> {
        let p = self.period()?;
        let n = self.orbit_number(current)?;
        Some(p * (n + 1) + self.time_at_periapsis)
    }

    pub fn t_last_p(&self, current: Nanotime) -> Option<Nanotime> {
        let p = self.period()?;
        let n = self.orbit_number(current)?;
        Some(p * n + self.time_at_periapsis)
    }

    pub fn focii(&self) -> [Vec2; 2] {
        let p = self.periapsis();
        let a = self.apoapsis();
        let c = (a + p) / 2.0;
        let u = (a - p).normalize();
        let d = self.semi_major_axis * self.eccentricity;
        [c + u * d, c - u * d]
    }

    pub fn center(&self) -> Vec2 {
        let p = self.periapsis();
        let a = self.apoapsis();
        (a + p) / 2.0
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

    pub fn is_consistent(&self, stamp: Nanotime) -> bool {
        let ta = self.ta_at_time(stamp);
        let ea = true_to_eccentric(ta, self.eccentricity);
        let ma = eccentric_to_mean(ea, self.eccentricity);
        let ea2 = match mean_to_eccentric(ma, self.eccentricity) {
            Some(e) => e,
            None => return false,
        };
        let ta2 = eccentric_to_true(ea2, self.eccentricity);
        (ta.as_f32() - ta2.as_f32()).abs() < 1E-3
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

pub fn to_aabbs(o: &Orbit) -> Vec<AABB> {
    let n = 30;

    let mut ret = Vec::new();
    let pos = (0..n)
        .map(|i| {
            let ta = 2.0 * PI * i as f32 / (n as f32 - 1.0);
            o.position_at(ta)
        })
        .collect::<Vec<_>>();

    for p in pos.windows(2) {
        let aabb = AABB::from_arbitrary(p[0], p[1]).padded(4.0);
        ret.push(aabb);
    }

    ret
}
