use starling::core::*;
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

fn apply(x: &Vec<f32>, func: impl Fn(f32) -> f32) -> Vec<f32> {
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

fn export_anomaly_conversions() -> Result<(), Box<dyn std::error::Error>> {
    let ecc = 0.8;

    let ta = linspace(-2.0 * PI, 2.0 * PI, 2000);
    let ea = apply(&ta, |x| {
        true_to_eccentric(Anomaly::with_ecc(ecc, x), ecc).as_f32()
    });
    let ma = apply(&ea, |x| {
        eccentric_to_mean(Anomaly::with_ecc(ecc, x), ecc).as_f32()
    });
    let ea2 = apply(&ma, |x| {
        mean_to_eccentric(Anomaly::with_ecc(ecc, x), ecc)
            .unwrap()
            .as_f32()
    });
    let ta2 = apply(&ea2, |x| {
        eccentric_to_true(Anomaly::with_ecc(ecc, x), ecc).as_f32()
    });

    write_csv(
        "anomaly.csv",
        &[
            ("ta", &ta),
            ("ea", &ea),
            ("ma", &ma),
            ("ea2", &ea2),
            ("ta2", &ta2),
        ],
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    export_sin_approx()?;
    export_anomaly_conversions()?;
    Ok(())
}
