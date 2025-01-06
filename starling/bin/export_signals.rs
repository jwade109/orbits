use starling::core::*;
use starling::examples::EARTH;
use starling::orbit::*;

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

// fn export_anomaly_conversions() -> Result<(), Box<dyn std::error::Error>> {
//     let ea = linspace(-PI, PI, 2000);
//     let mut signals = vec![("ea", ea.clone())];

//     for ecc in [0.2, 0.6, 0.9, 0.999, 0.9999] {
//         let ma = apply(&ea, |x| {
//             eccentric_to_mean(Anomaly::with_ecc(ecc, x), ecc).as_f32()
//         });
//         let ea2 = apply(&ma, |x| {
//             mean_to_eccentric(Anomaly::with_ecc(ecc, x), ecc)
//                 .map(|x| x.as_f32())
//                 .unwrap_or(f32::NAN)
//         });

//         let name = format!("ea({})", ecc);

//         signals.push((name, ea2));
//     }

//     write_csv("anomaly.csv", &signals)
// }

fn export_orbit_position() -> Result<(), Box<dyn std::error::Error>> {
    let orbit = Orbit::from_pv((400.0, 0.0), (0.0, 300.0), EARTH.mass, Nanotime::secs(5));

    let a = 200;

    let ftime = linspace(a as f32, -a as f32, 10000);
    let nt = apply(&ftime, |x| orbit.time_at_periapsis + Nanotime::secs_f32(x));

    let pos = apply(&nt, |x| orbit.pv_at_time(x).pos);
    let ta = apply(&nt, |x| orbit.ta_at_time(x).as_f32());
    let ea = apply(&nt, |x| orbit.ea_at_time(x).as_f32());
    let ma = apply(&nt, |x| orbit.ma_at_time(x).as_f32());

    let x = apply(&pos, |x| x.x);
    let y = apply(&pos, |x| x.y);
    let r = apply(&pos, |x| x.length());

    write_csv(
        "orbit.csv",
        &[
            ("t", &ftime),
            ("ta", &ta),
            ("ea", &ea),
            ("ma", &ma),
            ("r", &r),
        ],
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    export_sin_approx()?;
    // export_anomaly_conversions()?;
    export_orbit_position()?;
    Ok(())
}
