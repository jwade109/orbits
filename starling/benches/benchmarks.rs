use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use starling::orbital_luts::lookup_ta_from_ma;
use starling::prelude::*;

fn criterion_benchmark(c: &mut Criterion) {
    let mut s = c.benchmark_group("Small");

    s.measurement_time(std::time::Duration::from_secs(1));

    let o = SparseOrbit::new(
        12000.0,
        9000.0,
        0.3,
        Body::with_mass(1.0, 1000.0, 100000.0),
        Nanotime::zero(),
        false,
    )
    .unwrap();

    s.bench_function("pv_universal", |b| {
        b.iter(|| {
            let t = black_box(Nanotime::secs_f32(32.5));
            o.pv_universal(t).unwrap();
        })
    });

    s.bench_function("pv_lut", |b| {
        b.iter(|| {
            let t = black_box(Nanotime::secs_f32(32.5));
            o.pv_lut(t).unwrap();
        })
    });

    s.bench_function("position_at", |b| {
        b.iter(|| {
            let ta = black_box(0.43);
            o.position_at(ta);
        })
    });

    s.bench_function("velocity_at", |b| {
        b.iter(|| {
            let ta = black_box(0.43);
            o.velocity_at(ta);
        })
    });

    s.bench_function("eval_lut", |b| {
        lookup_ta_from_ma(0.0, 0.0);
        b.iter(|| {
            let ma = black_box(PI_64 * 1.2);
            let ecc = black_box(0.32);
            lookup_ta_from_ma(ma, ecc);
        })
    });

    s.finish();

    let mut g = c.benchmark_group("Sim");

    g.sample_size(1000);

    let (mut scenario, _) = stable_simulation();
    let planets = scenario.planets().clone();
    let mut t = Nanotime::zero();

    g.bench_function("scenario_sim", |b| {
        b.iter(|| {
            t += Nanotime::secs(10);
            Scenario::simulate(&mut scenario.orbiters, &planets, t, Nanotime::secs(20));
        })
    });

    g.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
