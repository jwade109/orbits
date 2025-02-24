#![allow(dead_code)]

use starling::prelude::*;

use starling::examples::make_earth;
use starling::file_export::{export_orbit_data, write_csv};
use starling::math::{apply, linspace, randvec};
use starling::orbits::{
    generate_chi_spline, stumpff_2, stumpff_3, universal_lagrange, SparseOrbit,
};

use std::path::Path;

fn export_orbit_position() -> Result<(), Box<dyn std::error::Error>> {
    let earth = make_earth();

    let initial = PV::new(randvec(100.0, 500.0), randvec(100.0, 400.0));

    let orbit = SparseOrbit::from_pv(initial, earth, Nanotime(0)).unwrap();

    _ = export_orbit_data(&orbit, Path::new("orbit.csv"));

    Ok(())
}

fn export_stumpff_functions() -> Result<(), Box<dyn std::error::Error>> {
    let x = linspace(-0.3, 0.3, 500000);

    let s2 = apply(&x, |x| stumpff_2(x));
    let s3 = apply(&x, |x| stumpff_3(x));

    write_csv(
        Path::new("stumpff.csv"),
        &[("x", &x), ("s2", &s2), ("s3", &s3)],
    )
}

fn write_chi_spline() -> Result<(), Box<dyn std::error::Error>> {
    let pv = PV::new((500.0, 200.0), (-11.0, 60.0));
    let orbit = SparseOrbit::from_pv(pv, make_earth(), Nanotime(0)).unwrap();

    let spline = generate_chi_spline(
        pv,
        make_earth().mu(),
        orbit.period().unwrap_or(Nanotime::secs(500)),
    )
    .unwrap();

    let start = spline.keys().first().unwrap().t;
    let end = spline.keys().last().unwrap().t;

    let teval = linspace(start, end, 10000);

    let spline = apply(&teval, |t| spline.sample(t).unwrap_or(f32::NAN));

    let data = apply(&teval, |t| {
        let (dat, res) = universal_lagrange(orbit.initial, Nanotime::secs_f32(t), orbit.body.mu());
        (dat, res.unwrap())
    });

    let actual = apply(&data, |d| d.1.chi);

    let error = spline
        .iter()
        .zip(actual.iter())
        .map(|(a, b)| b - a)
        .collect::<Vec<_>>();

    write_csv(
        std::path::Path::new("spline.csv"),
        &[
            ("t", &teval),
            ("spline", &spline),
            ("actual", &actual),
            ("error", &error),
        ],
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // export_sin_approx()?;
    // export_anomaly_conversions()?;
    // export_orbit_position()?;
    // export_stumpff_functions()?;
    write_chi_spline()?;
    Ok(())
}
