use crate::math::{apply, lerp, linspace, PI};
use crate::nanotime::Nanotime;
use crate::orbits::{wrap_pi_npi, Body, DenseOrbit, SparseOrbit};
use lazy_static::lazy_static;
use std::collections::HashMap;

fn get_orbit_with_ecc(ecc: f32) -> Vec<f32> {
    let a = 1000.0;
    let ra = a * (1.0 + ecc);
    let rp = a * (1.0 - ecc);
    let argp = 0.0;
    let body = Body {
        radius: 1.0,
        mass: 1000.0,
        soi: 100000.0,
    };
    let epoch = Nanotime::zero();
    let retrograde = false;
    let orbit = SparseOrbit::new(ra, rp, argp, body, epoch, retrograde).unwrap();
    let dense = DenseOrbit::new(&orbit).unwrap();

    let s = linspace(0.0, 1.0, 300);
    apply(&s, |s: f32| dense.sample_normalized(s) - s * 2.0 * PI)
}

const ECCENTRICITY_STEP: u8 = 3;

lazy_static! {
    pub static ref BIG_ORBITS: HashMap<u8, Vec<f32>> = HashMap::from_iter(
        (0..97)
            .step_by(ECCENTRICITY_STEP as usize)
            .map(|e| (e, get_orbit_with_ecc(e as f32 / 100.0)))
    );
}

pub fn lookup_ta_from_ma(ma: f32, ecc: f32) -> f32 {
    let ei = (ecc * 100.0) as u8;

    let el = ei - (ei % ECCENTRICITY_STEP);
    let eu = el + ECCENTRICITY_STEP;
    let s = ((ecc * 100.0) - (el as f32)) / ECCENTRICITY_STEP as f32;

    let lower = match BIG_ORBITS.get(&el) {
        Some(lut) => lut,
        None => return 0.0,
    };

    let upper = match BIG_ORBITS.get(&eu) {
        Some(lut) => lut,
        None => return 0.0,
    };

    assert_eq!(lower.len(), upper.len());

    let idx = ((ma / (2.0 * PI)) * (lower.len()) as f32) as usize;
    let tal = lower[idx % lower.len()];
    let tau = upper[idx % upper.len()];
    lerp(tal, tau, s)
}
