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

    let initial = PV::new((400.0, 0.0), (100.0, 10.0));

    let a = 10;

    let ftime = linspace(a as f32, -a as f32, 10000);

    // let (t_crazy, data_crazy) = (|| {
    //     for t in &ftime {
    //         let t = Nanotime::secs_f32(*t);
    //         if let Ok(data) = universal_lagrange(initial, t, earth.mu()) {
    //             if data.pv.pos.length() > 1000.0 {
    //                 return Some((t, data));
    //             }
    //         }
    //     }
    //     None
    // })()
    // .unwrap();

    // let a = 100;
    // let ftime = linspace(a as f32, -a as f32, 100000);

    let nt = apply(&ftime, |x| Nanotime::secs_f32(x));

    // let func = apply(&ftime, |x| {
    //     universal_kepler(
    //         x,
    //         initial.pos.length(),
    //         initial.vel.dot(initial.pos) / initial.pos.length(),
    //         data_crazy.alpha,
    //         t_crazy.to_secs(),
    //         earth.mu(),
    //     )
    // });

    let data = apply(&nt, |x| universal_lagrange(initial, x, earth.mu()));

    let x = apply(&data, |x| x.map(|d| d.pv.pos.x).unwrap_or(f32::NAN));
    let y = apply(&data, |x| x.map(|d| d.pv.pos.y).unwrap_or(f32::NAN));
    let alpha = apply(&data, |x| x.map(|d| d.chi_0).unwrap_or(f32::NAN));
    let chi_0 = apply(&data, |x| x.map(|d| d.chi_0).unwrap_or(f32::NAN));
    let chi = apply(&data, |x| x.map(|d| d.chi).unwrap_or(f32::NAN));
    // let g = apply(&data, |x| x.map(|d| d.g).unwrap_or(f32::NAN));

    write_csv("orbit.csv", &[("t", &ftime), ("x", &x), ("y", &y)])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    export_sin_approx()?;
    // export_anomaly_conversions()?;
    export_orbit_position()?;
    Ok(())
}
