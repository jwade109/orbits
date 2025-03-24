use crate::math::{apply, lerp, linspace, PI};
use crate::nanotime::Nanotime;
use crate::orbits::{Body, DenseOrbit, SparseOrbit};
use lazy_static::lazy_static;
use std::collections::HashMap;

const ECCENTRICITY_STEP: u8 = 1;
const N_SAMPLES: usize = 500;

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

    let s = linspace(0.0, 1.0, N_SAMPLES);
    apply(&s, |s: f32| dense.sample_normalized(s) - s * 2.0 * PI)
}

lazy_static! {
    pub static ref BIG_ORBITS: HashMap<u8, Vec<f32>> = HashMap::from_iter(
        (0..=98)
            .step_by(ECCENTRICITY_STEP as usize)
            .map(|e| (e, get_orbit_with_ecc(e as f32 / 100.0)))
    );
}

fn fmod(a: f32, n: f32) -> f32 {
    a - n * (a / n).floor()
}

pub fn lookup_ta_from_ma(ma: f32, ecc: f32) -> Option<f32> {
    let ma = fmod(ma, 2.0 * PI);

    let ei = (ecc * 100.0) as u8;

    let el = ei - (ei % ECCENTRICITY_STEP);
    let eu = el + ECCENTRICITY_STEP;
    let sy = ((ecc * 100.0) - (el as f32)) / ECCENTRICITY_STEP as f32;

    let lower = BIG_ORBITS.get(&el)?;
    let upper = BIG_ORBITS.get(&eu)?;

    let x1 = ((ma / (2.0 * PI)) * (N_SAMPLES - 1) as f32) as usize;
    let x2 = x1 + 1;

    let ma_x1 = (x1 as f32 / (N_SAMPLES - 1) as f32) * 2.0 * PI;
    let ma_x2 = (x2 as f32 / (N_SAMPLES - 1) as f32) * 2.0 * PI;

    let sx = (ma - ma_x1) / (ma_x2 - ma_x1);

    let x1y1 = lower[x1 % N_SAMPLES];
    let x1y2 = upper[x1 % N_SAMPLES];

    let x2y1 = lower[x2 % N_SAMPLES];
    let x2y2 = upper[x2 % N_SAMPLES];

    let p1 = lerp(x1y1, x1y2, sy);
    let p2 = lerp(x2y1, x2y2, sy);

    Some(lerp(p1, p2, sx) + ma)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_float_eq::assert_float_absolute_eq;

    #[test]
    fn lut_expected_values() {
        for ecc in linspace(0.0, 0.9, 100) {
            assert_float_absolute_eq!(lookup_ta_from_ma(0.0, ecc).unwrap(), 0.0, 1E-2);
            assert_float_absolute_eq!(lookup_ta_from_ma(PI, ecc).unwrap(), PI, 1E-2);
            assert_float_absolute_eq!(lookup_ta_from_ma(2.0 * PI, ecc).unwrap(), 0.0, 1E-2);
        }

        for ma in linspace(0.0, 1.95 * PI, 100) {
            assert_float_absolute_eq!(lookup_ta_from_ma(ma, 0.0).unwrap(), ma, 1E-2);
        }
    }
}
