use crate::core::{linspace, Nanotime};
use crate::orbits::universal::*;
use crate::planning::search_condition;
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

        if o.pv_at_time_fallible(epoch + Nanotime::secs(1)).is_err() {
            println!("SparseOrbit returned bad PV: {pv:?}\n  {o:?}");
            return None;
        }

        if e.is_nan() {
            println!("Bad orbit: {pv}");
            return None;
        }

        if let Some(p) = o.period() {
            if p == Nanotime(0) {
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
        let ret = Nanotime((t * 1E9) as i64);
        if ret == Nanotime(0) {
            return None;
        }
        Some(ret)
    }

    pub fn period_or(&self, fallback: Nanotime) -> Nanotime {
        self.period().unwrap_or(fallback)
    }

    pub fn pv_at_time(&self, stamp: Nanotime) -> PV {
        self.pv_at_time_fallible(stamp).unwrap_or(PV::zero())
    }

    pub fn pv_at_time_fallible(&self, stamp: Nanotime) -> Result<PV, ULData> {
        let tof = if let Some(p) = self.period() {
            (stamp - self.epoch) % p
        } else {
            stamp - self.epoch
        };
        let ul = universal_lagrange(self.initial, tof, self.body.mu());
        let sol = ul.1.ok_or(ul.0)?;
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

    pub fn t_last_p(&self, _current: Nanotime) -> Option<Nanotime> {
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

        // find all locations where ds/da goes from negative to positive
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::examples::{consistency_orbits, make_earth};
    use more_asserts::*;

    fn ncalc_period(orbit: &SparseOrbit) -> Option<(Nanotime, Nanotime)> {
        let dt = Nanotime::millis(10);
        let mut duration = Nanotime(0);
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
        let t1 = tspace(Nanotime(0), Nanotime::secs(n), n as u32);
        let t2 = tspace(Nanotime(0), Nanotime::secs(-n), n as u32);
        for t in t1.iter().chain(t2.iter()) {
            let pv = orbit.pv_at_time_fallible(*t);
            assert!(pv.is_ok(), "Failure at time {:?} - {:?}", t, pv);
        }
    }

    fn orbit_consistency_test(pv: PV, class: OrbitClass, body: Body) {
        println!("{}", pv);

        let orbit = SparseOrbit::from_pv(pv, body, Nanotime(0));

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
}
