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

    let orbit = Orbit::from_pv(initial, earth.mass, Nanotime(0));

    _ = export_orbit_data(&orbit, Path::new("orbit.csv"));

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    export_sin_approx()?;
    // export_anomaly_conversions()?;
    export_orbit_position()?;
    Ok(())
}
