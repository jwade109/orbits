use crate::core::Nanotime;
use crate::planning::search_condition;
use crate::pv::PV;
use splines::{Interpolation, Key, Spline};

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

// https://en.wikipedia.org/wiki/Universal_variable_formulation
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
            rootfinder::Interval::new(-9999.99, 9999.99),
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

pub fn tspace(start: Nanotime, end: Nanotime, nsamples: u32) -> Vec<Nanotime> {
    let dt = (end - start) / nsamples as i64;
    (0..nsamples).map(|i| start + dt * i as i64).collect()
}

type ChiSpline = Spline<f32, f32>;

pub fn generate_chi_spline(
    pv: impl Into<PV>,
    mu: f32,
    duration: Nanotime,
) -> Result<ChiSpline, ULError> {
    let tsample = tspace(Nanotime(0), duration, 500);
    let pv = pv.into();
    let x = tsample
        .to_vec()
        .iter()
        .map(|t| {
            let data = universal_lagrange(pv, *t, mu)?;
            let t = t.to_secs();
            let key = Key::new(t, data.chi, Interpolation::Linear);
            Ok(key)
        })
        .collect::<Result<Vec<_>, ULError>>()?;

    Ok(Spline::from_vec(x))
}
