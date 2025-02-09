use crate::aabb::AABB;
use crate::core::*;
use crate::planning::binary_search;
use crate::pv::*;
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

    pub fn mu(&self) -> f32 {
        self.mass * GRAVITATIONAL_CONSTANT
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
    pub initial: PV,
    pub epoch: Nanotime,
}

impl Orbit {
    pub fn from_pv(pv: impl Into<PV>, mass: f32, epoch: Nanotime) -> Option<Self> {
        let mu = mass * GRAVITATIONAL_CONSTANT;
        let pv: PV = pv.into();

        pv.filter_nan()?;

        let r3 = pv.pos.extend(0.0);
        let v3 = pv.vel.extend(0.0);
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
            initial: pv,
            epoch,
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

        if o.pv_at_time(epoch + Nanotime::secs(1))
            .filter_nan()
            .is_none()
        {
            println!("Orbit returned NaN PV: {pv:?}\n  {o:?}");
            return None;
        }

        if e.is_nan() {
            println!("Bad orbit: {pv}");
            return None;
        }

        Some(o)
    }

    pub fn circular(radius: f32, mass: f32, epoch: Nanotime, retrograde: bool) -> Self {
        let p = Vec2::new(radius, 0.0);
        let v = Vec2::new(0.0, (mass * GRAVITATIONAL_CONSTANT / radius).sqrt());
        Orbit {
            eccentricity: 0.0,
            semi_major_axis: radius,
            arg_periapsis: 0.0,
            retrograde,
            primary_mass: mass,
            time_at_periapsis: epoch,
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

    pub fn pv_at_time(&self, stamp: Nanotime) -> PV {
        universal_lagrange(
            self.initial,
            stamp - self.epoch,
            self.primary_mass * GRAVITATIONAL_CONSTANT,
        )
        .map(|t| t.pv)
        .unwrap_or(PV::zero())
    }

    pub fn pv_at_time_fallible(&self, stamp: Nanotime) -> Option<PV> {
        universal_lagrange(
            self.initial,
            stamp - self.epoch,
            self.primary_mass * GRAVITATIONAL_CONSTANT,
        )
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

    pub fn orbit_number(&self, stamp: Nanotime) -> Option<i64> {
        let p = self.period()?;
        let dt = stamp - self.time_at_periapsis;
        let n = dt.0.checked_div(p.0)?;
        Some(if dt.0 < 0 { n - 1 } else { n })
    }

    pub fn t_next_p(&self, current: Nanotime) -> Option<Nanotime> {
        if self.eccentricity >= 1.0 {
            return (self.time_at_periapsis >= current).then(|| self.time_at_periapsis);
        }
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

pub fn universal_kepler(chi: f32, r_0: f32, v_r0: f32, alpha: f32, delta_t: f32, mu: f32) -> f32 {
    let z = alpha * chi.powi(2);
    let first_term = r_0 * v_r0 / mu.sqrt() * chi.powi(2) * stumpff_2(z);
    let second_term = (1.0 - alpha * r_0) * chi.powi(3) * stumpff_3(z);
    let third_term = r_0 * chi;
    let fourth_term = mu.sqrt() * delta_t;
    first_term + second_term + third_term - fourth_term
}

// fn d_universal_d_chi(chi: f32, r_0: f32, v_r0: f32, alpha: f32, mu: f32) -> f32 {
//     let z = alpha * chi.powi(2);
//     let first_term = r_0 * v_r0 / mu.sqrt() * chi * (1.0 - z * stumpff_3(z));
//     let second_term = (1.0 - alpha * r_0) * chi.powi(2) * stumpff_2(z);
//     let third_term = r_0;
//     first_term + second_term + third_term
// }

#[derive(Debug, Clone, Copy)]
pub enum ULError {
    Solve,
    NaN,
}

#[derive(Debug, Copy, Clone)]
pub struct LangrangeCoefficients {
    pub s2: f32,
    pub s3: f32,
    pub f: f32,
    pub g: f32,
    pub fdot: f32,
    pub gdot: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct ULData {
    pub tof: Nanotime,
    pub pv: PV,
    pub alpha: f32,
    pub chi_0: f32,
    pub chi: f32,
    pub z: f32,
    pub lc: LangrangeCoefficients,
}

// https://orbital-mechanics.space/time-since-periapsis-and-keplers-equation/universal-lagrange-coefficients-example.html
pub fn universal_lagrange(
    initial: impl Into<PV>,
    tof: Nanotime,
    mu: f32,
) -> Result<ULData, ULError> {
    let initial = initial.into();
    let vec_r_0 = initial.pos;
    let vec_v_0 = initial.vel;

    let r_0 = vec_r_0.length();
    let v_r0 = vec_v_0.dot(vec_r_0) / r_0;

    let alpha = 2.0 / r_0 - vec_v_0.dot(vec_v_0) / mu;

    let delta_t = tof.to_secs();
    let chi_0: f32 = mu.sqrt() * alpha.abs() * delta_t;

    let chi = if tof == Nanotime(0) {
        0.0
    } else {
        rootfinder::root_bisection(
            &|x| universal_kepler(x as f32, r_0, v_r0, alpha, delta_t, mu).into(),
            rootfinder::Interval::new(-999999.99, 999999.99),
            None,
            None,
        )
        .map_err(|_| ULError::Solve)? as f32
    };

    let z = alpha * chi.powi(2);

    let lcoeffs = lagrange_coefficients(initial, chi, mu, tof);

    let pv = lagrange_pv(initial, &lcoeffs);

    Ok(ULData {
        tof,
        pv,
        alpha,
        chi_0,
        chi,
        z,
        lc: lcoeffs,
    })
}

pub fn lagrange_coefficients(
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

pub fn lagrange_pv(initial: impl Into<PV>, coeff: &LangrangeCoefficients) -> PV {
    let initial = initial.into();
    let vec_r = coeff.f * initial.pos + coeff.g * initial.vel;
    let vec_v = coeff.fdot * initial.pos + coeff.gdot * initial.vel;
    PV::new(vec_r, vec_v)
}

pub fn export_orbit_data(
    orbit: &Orbit,
    filename: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let orbit_data_path = std::path::Path::new("orbit_data/");
    std::fs::create_dir_all(&orbit_data_path)?;

    let a = orbit.period().unwrap_or(Nanotime::secs(30)).to_secs();

    let ftime = linspace(-a, a, 100000);

    let nt = apply(&ftime, |x| Nanotime::secs_f32(x));

    let data = apply(&nt, |x| {
        universal_lagrange(
            orbit.initial,
            x,
            orbit.primary_mass * GRAVITATIONAL_CONSTANT,
        )
    });

    let x = apply(&data, |x| x.map(|d| d.pv.pos.x).unwrap_or(f32::NAN));
    let y = apply(&data, |x| x.map(|d| d.pv.pos.y).unwrap_or(f32::NAN));
    let vx = apply(&data, |x| x.map(|d| d.pv.vel.x).unwrap_or(f32::NAN));
    let vy = apply(&data, |x| x.map(|d| d.pv.vel.y).unwrap_or(f32::NAN));
    let r = apply(&data, |x| x.map(|d| d.pv.pos.length()).unwrap_or(f32::NAN));
    let z = apply(&data, |x| x.map(|d| d.z).unwrap_or(f32::NAN));
    let f = apply(&data, |x| x.map(|d| d.lc.f).unwrap_or(f32::NAN));
    let g = apply(&data, |x| x.map(|d| d.lc.g).unwrap_or(f32::NAN));
    let fdot = apply(&data, |x| x.map(|d| d.lc.fdot).unwrap_or(f32::NAN));
    let gdot = apply(&data, |x| x.map(|d| d.lc.gdot).unwrap_or(f32::NAN));

    write_csv(
        &orbit_data_path.join(filename),
        &[
            ("t", &ftime),
            ("x", &x),
            ("y", &y),
            ("vx", &vx),
            ("vy", &vy),
            ("r", &r),
            ("z", &z),
            ("f", &f),
            ("g", &g),
            ("fdot", &fdot),
            ("gdot", &gdot),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn universal_lagrange_example() {
        let vec_r_0 = Vec2::new(7000.0, -12124.0);
        let vec_v_0 = Vec2::new(2.6679, 4.6210);
        let mu = 3.986004418E5;

        let tof = Nanotime::secs(3600);

        let res = super::universal_lagrange((vec_r_0, vec_v_0), tof, mu).unwrap();

        assert_eq!(
            res.pv,
            PV::new((-3297.7869, 7413.3867), (-8.297602, -0.9640651))
        );
    }

    #[test]
    fn stumpff() {
        assert_eq!(stumpff_2(-20.0), 2.1388736);
        assert_eq!(stumpff_2(-5.0), 0.74633473);
        assert_eq!(stumpff_2(-1.0), 0.5430807);
        assert_eq!(stumpff_2(-1E-6), 0.50000006);
        assert_eq!(stumpff_2(-1E-12), 0.5);
        assert_eq!(stumpff_2(0.0), 0.5);
        assert_eq!(stumpff_2(1E-12), 0.5);
        assert_eq!(stumpff_2(1E-6), 0.49999997);
        assert_eq!(stumpff_2(1.0), 0.45969772);
        assert_eq!(stumpff_2(5.0), 0.32345456);
        assert_eq!(stumpff_2(20.0), 0.061897416);

        assert_eq!(stumpff_3(-20.0), 0.43931928);
        assert_eq!(stumpff_3(-1E-12), 0.16666667);
        assert_eq!(stumpff_3(0.0), 0.16666667);
        assert_eq!(stumpff_3(1E-12), 0.16666667);
        assert_eq!(stumpff_3(20.0), 0.060859215);
    }

    #[test]
    fn orbit_construction() {
        const TEST_POSITION: Vec2 = Vec2::new(500.0, 300.0);
        const TEST_VELOCITY: Vec2 = Vec2::new(-200.0, 0.0);

        let mass = 1000.0;

        let o1 = Orbit::from_pv((TEST_POSITION, TEST_VELOCITY), mass, Nanotime(0)).unwrap();
        let o2 = Orbit::from_pv((TEST_POSITION, -TEST_VELOCITY), mass, Nanotime(0)).unwrap();

        let true_h = TEST_POSITION.extend(0.0).cross(TEST_VELOCITY.extend(0.0)).z;

        assert_relative_eq!(o1.angular_momentum(), true_h);
        assert!(o1.angular_momentum() > 0.0);
        assert!(!o1.retrograde);

        assert_relative_eq!(o2.angular_momentum(), true_h);
        assert!(o1.angular_momentum() > 0.0);
        assert!(o2.retrograde);

        assert_eq!(o1.period().unwrap(), o2.period().unwrap());

        for i in -5..5 {
            let t = o1.period().unwrap() * i;
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
