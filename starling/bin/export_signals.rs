use starling::core::*;
use starling::examples::make_earth;
use starling::orbit::*;
use starling::pv::*;

use std::path::Path;

fn export_sin_approx() -> Result<(), Box<dyn std::error::Error>> {
    let x = linspace(-4.0 * PI, 4.0 * PI, 2000);
    let sinx = apply(&x, |x| x.sin());
    let approx = apply(&x, |x| bhaskara_sin_approx(x));
    let extend = apply(&x, |x| {
        let xp = (x.abs() + PI) % (2.0 * PI) - PI;
        let y = bhaskara_sin_approx(xp);
        if x < 0.0 {
            -y
        } else {
            y
        }
    });

    write_csv(
        Path::new("sin_approx.csv"),
        &[
            ("x", &x),
            ("sinx", &sinx),
            ("approx", &approx),
            ("ex", &extend),
        ],
    )
}

fn export_orbit_position() -> Result<(), Box<dyn std::error::Error>> {
    let earth = make_earth();

    let initial = PV::new(randvec(100.0, 500.0), randvec(100.0, 400.0));

    let orbit = Orbit::from_pv(initial, earth, Nanotime(0)).unwrap();

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

fn write_univ_kepler() -> Result<(), Box<dyn std::error::Error>> {
    let pv = PV::new((-4246.739, 1152.7261), (-10.610792, 80.369736));
    // let pv = PV::new(randvec(100.0, 500.0), randvec(100.0, 400.0));

    let mu = 1000.0;
    let chi_0 = linspace(-1.0, 1.0, 10000);
    let r_0 = pv.pos.length();
    let v_r0 = pv.vel.length();
    let alpha = 2.0 / r_0 - pv.vel.dot(pv.vel) / mu;

    let chi = apply(&chi_0, |chi| {
        universal_kepler(chi, r_0, v_r0, alpha, 120.0, mu)
    });

    write_csv(
        std::path::Path::new("chi.csv"),
        &[("chi_0", &chi_0), ("chi", &chi)],
    )
}

fn write_chi_spline() -> Result<(), Box<dyn std::error::Error>> {
    let pv = PV::new((500.0, 200.0), (-11.0, 60.0));
    let orbit = Orbit::from_pv(pv, make_earth(), Nanotime(0)).unwrap();

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
        universal_lagrange(orbit.initial, Nanotime::secs_f32(t), orbit.body.mu(), None).unwrap()
    });

    let actual = apply(&data, |d| d.chi);

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
    write_univ_kepler()?;
    write_chi_spline()?;
    Ok(())
}
