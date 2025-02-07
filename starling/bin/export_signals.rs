use starling::core::*;
use starling::examples::make_earth;
use starling::orbit::*;
use starling::pv::PV;

fn write_csv(filename: &str, signals: &[(&str, &[f32])]) -> Result<(), Box<dyn std::error::Error>> {
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

fn apply<T: Copy, R>(x: &Vec<T>, func: impl Fn(T) -> R) -> Vec<R> {
    x.iter().map(|x| func(*x)).collect()
}

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
        "sin_approx.csv",
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

    // let initial = PV::new((293.74924, -161.09222), (320.1136, 18.492138));
    // let initial = PV::new((-259.96744, -195.29225), (-290.76135, 159.96176));

    dbg!(initial);

    let a = 20;

    let ftime = linspace(a as f32, -a as f32, 100000);

    let nt = apply(&ftime, |x| Nanotime::secs_f32(x));

    let data = apply(&nt, |x| universal_lagrange(initial, x, earth.mu()));

    dbg!(data[0]);

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
        "orbit.csv",
        &[
            ("t", &ftime),
            ("x", &x),
            ("y", &y),
            ("vx", &vx),
            ("vy", &vy),
            ("r", &r),
            ("z", &z),
        ],
    )?;

    write_csv(
        "lagrange.csv",
        &[
            ("t", &ftime),
            ("f", &f),
            ("g", &g),
            ("fdot", &fdot),
            ("gdot", &gdot),
        ],
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    export_sin_approx()?;
    // export_anomaly_conversions()?;
    export_orbit_position()?;
    Ok(())
}
