use crate::math::{apply, linspace};
use crate::nanotime::Nanotime;
use crate::orbits::{universal_lagrange, SparseOrbit};

pub fn write_csv(
    filename: &std::path::Path,
    signals: &[(&str, &[f32])],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = csv::Writer::from_path(filename)?;

    let titles = signals.iter().map(|s| s.0);

    writer.write_record(titles)?;

    for i in 0.. {
        let iter = signals
            .iter()
            .map(|s| s.1.get(i))
            .map(|s| s.map(|e| format!("{:0.5}", e)))
            .collect::<Option<Vec<_>>>();
        if let Some(row) = iter {
            writer.write_record(row)?;
        } else {
            break;
        }
    }

    writer.flush()?;

    Ok(())
}

pub fn export_orbit_data(
    orbit: &SparseOrbit,
    filename: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let orbit_data_path = std::path::Path::new("orbit_data/");
    std::fs::create_dir_all(&orbit_data_path)?;

    let a = orbit.period().unwrap_or(Nanotime::secs(30)).to_secs();

    let ftime = linspace(-a, a, 100000);

    let nt = apply(&ftime, |x| Nanotime::secs_f32(x));

    let data = apply(&nt, |x| {
        universal_lagrange(orbit.initial, x, orbit.body().mu())
    });

    let x = apply(&data, |x| x.1.map(|d| d.pv.pos.x).unwrap_or(f32::NAN));
    let y = apply(&data, |x| x.1.map(|d| d.pv.pos.y).unwrap_or(f32::NAN));
    let vx = apply(&data, |x| x.1.map(|d| d.pv.vel.x).unwrap_or(f32::NAN));
    let vy = apply(&data, |x| x.1.map(|d| d.pv.vel.y).unwrap_or(f32::NAN));
    let r = apply(&data, |x| {
        x.1.map(|d| d.pv.pos.length()).unwrap_or(f32::NAN)
    });
    let z = apply(&data, |x| x.1.map(|d| d.z).unwrap_or(f32::NAN));
    let f = apply(&data, |x| x.1.map(|d| d.lc.f).unwrap_or(f32::NAN));
    let g = apply(&data, |x| x.1.map(|d| d.lc.g).unwrap_or(f32::NAN));
    let fdot = apply(&data, |x| x.1.map(|d| d.lc.fdot).unwrap_or(f32::NAN));
    let gdot = apply(&data, |x| x.1.map(|d| d.lc.gdot).unwrap_or(f32::NAN));

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
