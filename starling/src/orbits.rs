use crate::math::{linspace, tspace, PI};
use crate::nanotime::Nanotime;
use crate::planning::search_condition;
use crate::pv::PV;
use glam::f32::Vec2;
use serde::{Deserialize, Serialize};
use splines::{Interpolation, Key, Spline};

pub fn hyperbolic_range_ta(ecc: f32) -> f32 {
    (-1.0 / ecc).acos()
}

fn wrap_pi_npi(x: f32) -> f32 {
    f32::atan2(x.sin(), x.cos())
}

#[derive(Debug, Clone, Copy)]
enum Anomaly {
    Elliptical(f32),
    Parabolic(f32),
    Hyperbolic(f32),
}

impl Anomaly {
    fn with_ecc(ecc: f32, anomaly: f32) -> Self {
        if ecc > 1.0 {
            Anomaly::Hyperbolic(anomaly)
        } else if ecc == 1.0 {
            Anomaly::Parabolic(anomaly)
        } else {
            Anomaly::Elliptical(wrap_pi_npi(anomaly))
        }
    }

    fn as_f32(&self) -> f32 {
        match self {
            Anomaly::Elliptical(v) => *v,
            Anomaly::Parabolic(v) => *v,
            Anomaly::Hyperbolic(v) => *v,
        }
    }
}

fn true_to_eccentric(true_anomaly: Anomaly, ecc: f32) -> Anomaly {
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

fn eccentric_to_mean(eccentric_anomaly: Anomaly, ecc: f32) -> Anomaly {
    match eccentric_anomaly {
        Anomaly::Elliptical(v) => Anomaly::Elliptical(v - ecc * v.sin()),
        Anomaly::Hyperbolic(v) => Anomaly::Hyperbolic(ecc * v.sinh() - v),
        Anomaly::Parabolic(v) => Anomaly::Parabolic(v + v.powi(3) / 3.0),
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

        if o.pv_at_time_fallible(epoch + Nanotime::secs(1)).is_err() {
            println!("SparseOrbit returned bad PV: {pv:?}\n  {o:?}");
            return None;
        }

        if e.is_nan() {
            println!("Bad orbit: {pv}");
            return None;
        }

        if let Some(p) = o.period() {
            if p == Nanotime::zero() {
                println!("SparseOrbit returned orbit with zero period: {pv:?}\n  {o:?}");
                return None;
            }
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
            time_at_periapsis: None,
        }
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
        let ret = Nanotime::nanos((t * 1E9) as i64);
        if ret == Nanotime::zero() {
            return None;
        }
        Some(ret)
    }

    pub fn period_or(&self, fallback: Nanotime) -> Nanotime {
        self.period().unwrap_or(fallback)
    }

    pub fn pv_at_time(&self, stamp: Nanotime) -> PV {
        self.pv_at_time_fallible(stamp).unwrap_or(PV::new(
            Vec2::splat(f32::INFINITY),
            Vec2::splat(f32::INFINITY),
        ))
    }

    pub fn pv_at_time_fallible(&self, stamp: Nanotime) -> Result<PV, ULData> {
        let tof = if let Some(p) = self.period() {
            (stamp - self.epoch) % p
        } else {
            stamp - self.epoch
        };
        let ul = universal_lagrange(self.initial, tof, self.body.mu());
        let sol = ul.1.ok_or(ul.0)?;
        if sol.pv.pos.length() > 3.0 * self.body.soi {
            return Err(ul.0);
        }
        Ok(sol.pv.filter_numerr().ok_or(ul.0)?)
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
        let n = dt.inner().checked_div(p.inner())?;
        Some(if dt.inner() < 0 { n - 1 } else { n })
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

    pub fn nearest_approach(&self, other: SparseOrbit) -> Option<Vec<f32>> {
        // distance between orbits along a ray cast from planet object
        let separation = |a: f32| self.radius_at_angle(a) - other.radius_at_angle(a);

        // trend of separation (proportional to ds/da)
        // positive if separation is growing with angle
        let derivative = |a: f32| {
            let da = 0.03;
            separation(a - da) - separation(a + da)
        };

        let aeval = linspace(0.0, 2.0 * PI, 100);

        let c1 = |a: f32| derivative(a) > 0.0;
        let c2 = |a: f32| derivative(a) < 0.0;
        let c3 = |a: f32| separation(a) > 0.0;
        let c4 = |a: f32| separation(a) < 0.0;

        let mut ret = vec![];

        for a in aeval.windows(2) {
            match search_condition::<f32>(a[0], a[1], 1E-6, &c1) {
                Ok(Some(found)) => ret.push(found),
                Ok(None) => (),
                Err(e) => {
                    dbg!(e);
                    return None;
                }
            }
            match search_condition::<f32>(a[0], a[1], 1E-6, &c2) {
                Ok(Some(found)) => ret.push(found),
                Ok(None) => (),
                Err(e) => {
                    dbg!(e);
                    return None;
                }
            }
            match search_condition::<f32>(a[0], a[1], 1E-6, &c3) {
                Ok(Some(found)) => ret.push(found),
                Ok(None) => (),
                Err(e) => {
                    dbg!(e);
                    return None;
                }
            }
            match search_condition::<f32>(a[0], a[1], 1E-6, &c4) {
                Ok(Some(found)) => ret.push(found),
                Ok(None) => (),
                Err(e) => {
                    dbg!(e);
                    return None;
                }
            }
        }
        Some(ret)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrbitClass {
    Circular,
    NearCircular,
    Elliptical,
    HighlyElliptical,
    Parabolic,
    Hyperbolic,
    VeryThin,
}

// https://www.coursesidekick.com/mathematics/441994

// https://orbital-mechanics.space/time-since-periapsis-and-keplers-equation/universal-variables.html

// 2nd stumpff function
// aka C(z)
pub fn stumpff_2(z: f32) -> f32 {
    let midwidth = 0.01;
    if z > midwidth {
        (1.0 - z.sqrt().cos()) / z
    } else if z < -midwidth {
        ((-z).sqrt().cosh() - 1.0) / -z
    } else {
        0.5 - 0.04 * z
    }
}

// 3rd stumpff function
// aka S(z)
pub fn stumpff_3(z: f32) -> f32 {
    let midwidth = 0.01;
    if z > midwidth {
        (z.sqrt() - z.sqrt().sin()) / z.powf(1.5)
    } else if z < -midwidth {
        ((-z).sqrt().sinh() - (-z).sqrt()) / (-z).powf(1.5)
    } else {
        -0.00833 * z + 1.0 / 6.0
    }
}

fn universal_kepler(chi: f32, r_0: f32, v_r0: f32, alpha: f32, delta_t: f32, mu: f32) -> f32 {
    let z = alpha * chi.powi(2);
    let first_term = r_0 * v_r0 / mu.sqrt() * chi.powi(2) * stumpff_2(z);
    let second_term = (1.0 - alpha * r_0) * chi.powi(3) * stumpff_3(z);
    let third_term = r_0 * chi;
    let fourth_term = mu.sqrt() * delta_t;
    first_term + second_term + third_term - fourth_term
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct LangrangeCoefficients {
    #[allow(unused)]
    pub(crate) s2: f32,
    #[allow(unused)]
    pub(crate) s3: f32,
    pub(crate) f: f32,
    pub(crate) g: f32,
    pub(crate) fdot: f32,
    pub(crate) gdot: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct ULData {
    pub(crate) initial: PV,
    pub(crate) tof: Nanotime,
    pub(crate) mu: f32,
    pub(crate) r_0: f32,
    pub(crate) v_r0: f32,
    pub(crate) chi_0: f32,
    pub(crate) alpha: f32,
}

impl ULData {
    fn new(initial: impl Into<PV>, tof: Nanotime, mu: f32) -> Self {
        let initial = initial.into();
        let r_0 = initial.pos.length();
        let alpha = 2.0 / r_0 - initial.vel.dot(initial.vel) / mu;
        ULData {
            initial,
            tof,
            mu,
            r_0,
            v_r0: initial.vel.dot(initial.pos) / r_0,
            alpha,
            chi_0: mu.sqrt() * alpha.abs() * tof.to_secs(),
        }
    }

    fn universal_kepler(&self, chi: f32) -> f32 {
        universal_kepler(
            chi,
            self.r_0,
            self.v_r0,
            self.alpha,
            self.tof.to_secs(),
            self.mu,
        )
    }

    fn solve(&self) -> Option<ULResults> {
        let radius = 800.0;
        let chi_min = self.chi_0 - radius;
        let chi_max = self.chi_0 + radius;
        let chi = if self.tof == Nanotime::zero() {
            0.0
        } else {
            match rootfinder::root_bisection(
                &|x: f64| self.universal_kepler(x as f32) as f64,
                rootfinder::Interval::new(chi_min as f64, chi_max as f64),
                None,
                None,
            ) {
                Ok(x) => x as f32,
                Err(_) => {
                    return None;
                }
            }
        };

        if chi == chi_min || chi == chi_max {
            return None;
        }

        ULResults::new(chi, &self)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ULResults {
    pub(crate) pv: PV,
    pub chi: f32,
    pub(crate) z: f32,
    pub(crate) lc: LangrangeCoefficients,
}

impl ULResults {
    fn new(chi: f32, data: &ULData) -> Option<Self> {
        let z = data.alpha * chi.powi(2);
        let lcoeffs = lagrange_coefficients(data.initial, chi, data.mu, data.tof);
        let pv = lagrange_pv(data.initial, &lcoeffs).filter_numerr()?;
        Some(ULResults {
            pv,
            chi,
            z,
            lc: lcoeffs,
        })
    }
}

// https://en.wikipedia.org/wiki/Universal_variable_formulation
// https://orbital-mechanics.space/time-since-periapsis-and-keplers-equation/universal-lagrange-coefficients-example.html
pub fn universal_lagrange(
    initial: impl Into<PV>,
    tof: Nanotime,
    mu: f32,
) -> (ULData, Option<ULResults>) {
    let data = ULData::new(initial, tof, mu);
    (data, data.solve())
}

pub(crate) fn lagrange_coefficients(
    initial: impl Into<PV>,
    chi: f32,
    mu: f32,
    dt: Nanotime,
) -> LangrangeCoefficients {
    let initial = initial.into();
    let vec_r_0 = initial.pos;
    let vec_v_0 = initial.vel;

    let r_0 = vec_r_0.length();

    let alpha = 2.0 / r_0 - vec_v_0.dot(vec_v_0) / mu;

    let delta_t = dt.to_secs();

    let z = alpha * chi.powi(2);

    let s2 = stumpff_2(z);
    let s3 = stumpff_3(z);

    let f = 1.0 - chi.powi(2) / r_0 * s2;
    let g = delta_t - chi.powi(3) / mu.sqrt() * s3;

    let vec_r = f * vec_r_0 + g * vec_v_0;
    let r = vec_r.length();

    let fdot = chi * mu.sqrt() / (r * r_0) * (z * s3 - 1.0);
    let gdot = 1.0 - chi.powi(2) / r * s2;

    LangrangeCoefficients {
        s2,
        s3,
        f,
        g,
        fdot,
        gdot,
    }
}

pub(crate) fn lagrange_pv(initial: impl Into<PV>, coeff: &LangrangeCoefficients) -> PV {
    let initial = initial.into();
    let vec_r = coeff.f * initial.pos + coeff.g * initial.vel;
    let vec_v = coeff.fdot * initial.pos + coeff.gdot * initial.vel;
    PV::new(vec_r, vec_v)
}

#[allow(unused)]
pub type ChiSpline = Spline<f32, f32>;

#[allow(unused)]
pub fn generate_chi_spline(pv: impl Into<PV>, mu: f32, duration: Nanotime) -> Option<ChiSpline> {
    let tsample = tspace(Nanotime::zero(), duration, 500);
    let pv = pv.into();
    let x = tsample
        .to_vec()
        .iter()
        .map(|t| {
            let (_, res) = universal_lagrange(pv, *t, mu);
            let res = res?;
            let t = t.to_secs();
            let key = Key::new(t, res.chi, Interpolation::Linear);
            Some(key)
        })
        .collect::<Option<Vec<_>>>()?;

    Some(Spline::from_vec(x))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::examples::{consistency_orbits, make_earth, stable_simulation};
    use crate::pv::PV;
    use approx::assert_relative_eq;
    use more_asserts::*;

    #[test]
    fn serialization() {
        let scenario = stable_simulation().0;
        let yaml = serde_yaml::to_string(&scenario).unwrap();
        let scenario = stable_simulation().0;
        let toml = toml::to_string(&scenario).unwrap();
        dbg!(yaml);
        dbg!(toml);
    }

    fn ncalc_period(orbit: &SparseOrbit) -> Option<(Nanotime, Nanotime)> {
        let dt = Nanotime::millis(10);
        let mut duration = Nanotime::zero();
        let pv0 = orbit.initial;
        let mut d_prev = 0.0;
        let mut was_decreasing = false;
        while duration < Nanotime::secs(10000) {
            duration += dt;
            let t = orbit.epoch + duration;
            let pv = orbit.pv_at_time_fallible(t).ok()?;
            let d = pv.pos.distance(pv0.pos);
            let increasing = d > d_prev;
            d_prev = d;

            let aligned = pv0.vel.dot(pv.vel) > 0.0;

            if d < 20.0 && aligned && increasing && was_decreasing {
                return Some((t - dt * 5, t));
            }

            was_decreasing = !increasing;
        }
        None
    }

    fn physics_based_smoketest(orbit: &SparseOrbit) {
        // TODO
        if orbit.class() == OrbitClass::VeryThin {
            return;
        }

        let mut particle = orbit.initial;
        let dt = Nanotime::millis(2);
        let mut t = orbit.epoch;

        let mut last_error = 0.0;
        let max_error_growth = 1.0;
        let mut previous = PV::zero();

        while t < Nanotime::secs(100) {
            t += dt;
            let porbit = match orbit.pv_at_time_fallible(t) {
                Ok(p) => p,
                Err(ul) => {
                    assert!(false, "Bad orbital position at {:?} - {:?}", t, ul);
                    return;
                }
            };
            let r2 = particle.pos.length_squared();
            let a = -orbit.body.mu() / r2 * particle.pos.normalize_or_zero();
            particle.vel += a * dt.to_secs();
            particle.pos += particle.vel * dt.to_secs();

            let error = porbit.pos.distance(particle.pos);
            let max_error = last_error + max_error_growth;
            assert_le!(
                error,
                max_error,
                "Deviation exceeded at {:?}, prev error {:0.3}\
                \n  Particle:       {:?}\
                \n  Previous orbit: {:?}\
                \n  Bad orbit pos:  {:?}",
                t,
                last_error,
                particle,
                previous,
                porbit,
            );
            last_error = error;
            previous = porbit;
        }

        println!("Max error: {:0.3}", last_error);
    }

    fn assert_defined_for_large_time_range(orbit: &SparseOrbit) {
        // TODO apply this to hyperbolic orbits too!
        match orbit.class() {
            OrbitClass::Hyperbolic | OrbitClass::Parabolic => {
                return;
            }
            _ => (),
        }

        let n = 10000;
        let t1 = tspace(Nanotime::zero(), Nanotime::secs(n), n as u32);
        let t2 = tspace(Nanotime::zero(), Nanotime::secs(-n), n as u32);
        for t in t1.iter().chain(t2.iter()) {
            let pv = orbit.pv_at_time_fallible(*t);
            assert!(pv.is_ok(), "Failure at time {:?} - {:?}", t, pv);
        }
    }

    fn orbit_consistency_test(pv: PV, class: OrbitClass, body: Body) {
        println!("{}", pv);

        let orbit = SparseOrbit::from_pv(pv, body, Nanotime::zero());

        assert!(orbit.is_some());

        let orbit = orbit.unwrap();

        assert_eq!(
            orbit.pv_at_time_fallible(orbit.epoch).ok(),
            Some(orbit.initial)
        );

        if let Some(((min, max), period)) = ncalc_period(&orbit).zip(orbit.period()) {
            dbg!((min, max, period));
            let tol = Nanotime::secs(1);
            assert_le!(min - tol, period, "Period too small: {:?}", orbit);
            assert_ge!(max + tol, period, "Period too big: {:?}", orbit);
        }

        assert_eq!(orbit.class(), class);
        dbg!(orbit.class());

        physics_based_smoketest(&orbit);
        assert_defined_for_large_time_range(&orbit);
    }

    #[test]
    fn orbit_001() {
        orbit_consistency_test(
            PV::new((669.058, -1918.289), (74.723, 60.678)),
            OrbitClass::Elliptical,
            Body::new(63.0, 1000.0, 15000.0),
        );
    }

    #[test]
    fn orbit_002() {
        orbit_consistency_test(
            PV::new((430.0, 230.0), (-50.14, 40.13)),
            OrbitClass::Elliptical,
            Body::new(63.0, 1000.0, 15000.0),
        );
    }

    #[test]
    fn orbit_003() {
        orbit_consistency_test(
            PV::new((0.0, -222.776), (333.258, 0.000)),
            OrbitClass::Hyperbolic,
            Body::new(63.0, 1000.0, 15000.0),
        );
    }

    #[test]
    fn orbit_004() {
        orbit_consistency_test(
            PV::new((1520.323, 487.734), (-84.935, 70.143)),
            OrbitClass::Elliptical,
            Body::new(63.0, 1000.0, 15000.0),
        );
    }

    #[test]
    fn orbit_005() {
        orbit_consistency_test(
            PV::new((5535.6294, -125.794685), (-66.63476, 16.682587)),
            OrbitClass::Hyperbolic,
            Body::new(63.0, 1000.0, 15000.0),
        );
    }

    #[test]
    fn orbit_006() {
        orbit_consistency_test(
            PV::new((65.339584, 1118.9651), (-138.84702, -279.47888)),
            OrbitClass::Hyperbolic,
            Body::new(63.0, 1000.0, 15000.0),
        );
    }

    #[test]
    fn orbit_007() {
        orbit_consistency_test(
            PV::new((-1856.4648, -1254.9697), (216.31313, -85.84622)),
            OrbitClass::Hyperbolic,
            Body::new(63.0, 1000.0, 15000.0),
        );
    }

    #[test]
    fn orbit_008() {
        orbit_consistency_test(
            PV::new((-72.39488, 662.50507), (3.4047441, 71.81263)),
            OrbitClass::Hyperbolic,
            Body::new(22.0, 10.0, 800.0),
        );
    }

    #[test]
    fn grid_orbits() {
        let orbits = consistency_orbits(make_earth());
        for orbit in &orbits[0..120] {
            orbit_consistency_test(orbit.initial, orbit.class(), orbit.body);
        }
    }

    #[test]
    fn universal_lagrange_example() {
        let vec_r_0 = Vec2::new(7000.0, -12124.0);
        let vec_v_0 = Vec2::new(2.6679, 4.6210);
        let mu = 3.986004418E5;

        let tof = Nanotime::secs(3600);

        let (_, res) = super::universal_lagrange((vec_r_0, vec_v_0), tof, mu);

        assert_eq!(
            res.unwrap().pv,
            PV::new((-3297.7869, 7413.3867), (-8.297602, -0.9640651))
        );
    }

    #[test]
    fn stumpff() {
        assert_relative_eq!(stumpff_2(-20.0), 2.1388736);
        assert_relative_eq!(stumpff_2(-5.0), 0.74633473);
        assert_relative_eq!(stumpff_2(-1.0), 0.5430807);
        assert_relative_eq!(stumpff_2(-1E-6), 0.50000006);
        assert_relative_eq!(stumpff_2(-1E-12), 0.5);
        assert_relative_eq!(stumpff_2(0.0), 0.5);
        assert_relative_eq!(stumpff_2(1E-12), 0.5);
        assert_relative_eq!(stumpff_2(1E-6), 0.49999997);
        assert_relative_eq!(stumpff_2(1.0), 0.45969772);
        assert_relative_eq!(stumpff_2(5.0), 0.32345456);
        assert_relative_eq!(stumpff_2(20.0), 0.061897416);

        assert_relative_eq!(stumpff_3(-20.0), 0.43931928);
        assert_relative_eq!(stumpff_3(-1E-12), 0.16666667);
        assert_relative_eq!(stumpff_3(0.0), 0.16666667);
        assert_relative_eq!(stumpff_3(1E-12), 0.16666667);
        assert_relative_eq!(stumpff_3(20.0), 0.060859215);
    }

    #[test]
    fn bad_orbit() {
        let pv = PV::new((825.33563, 564.6425), (200.0, 230.0));
        let body = Body {
            radius: 63.0,
            mass: 1000.0,
            soi: 15000.0,
        };

        let data = ULData::new(pv, Nanotime::secs(1), body.mu());

        let res = data.solve();

        dbg!(data);
        dbg!(res);
    }

    #[test]
    fn orbit_construction() {
        const TEST_POSITION: Vec2 = Vec2::new(500.0, 300.0);
        const TEST_VELOCITY: Vec2 = Vec2::new(-200.0, 0.0);

        let body = Body {
            radius: 100.0,
            mass: 1000.0,
            soi: 10000.0,
        };

        let o1 =
            SparseOrbit::from_pv((TEST_POSITION, TEST_VELOCITY), body, Nanotime::zero()).unwrap();
        let o2 =
            SparseOrbit::from_pv((TEST_POSITION, -TEST_VELOCITY), body, Nanotime::zero()).unwrap();

        let true_h = TEST_POSITION.extend(0.0).cross(TEST_VELOCITY.extend(0.0)).z;

        assert_relative_eq!(o1.angular_momentum(), true_h);
        assert!(o1.angular_momentum() > 0.0);
        assert!(!o1.retrograde);

        assert_relative_eq!(o2.angular_momentum(), true_h);
        assert!(o1.angular_momentum() > 0.0);
        assert!(o2.retrograde);

        assert_eq!(o1.period().unwrap(), o2.period().unwrap());

        // TODO make this better
        for i in [0] {
            let t = o1.period().unwrap() * i;
            println!("{t:?} {} {}", o1.pv_at_time(t), o2.pv_at_time(t));
            assert_relative_eq!(
                o1.pv_at_time(t).pos.x,
                o2.pv_at_time(t).pos.x,
                epsilon = 0.5
            );
            assert_relative_eq!(
                o1.pv_at_time(t).pos.y,
                o2.pv_at_time(t).pos.y,
                epsilon = 0.5
            );
        }
    }
}
