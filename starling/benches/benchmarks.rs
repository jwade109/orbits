use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use starling::orbital_luts::lookup_ta_from_ma;
use starling::orbits::generate_chi_spline;
use starling::prelude::*;

fn criterion_benchmark(c: &mut Criterion) {
    let mut s = c.benchmark_group("Small");

    s.measurement_time(std::time::Duration::from_secs(1));

    let o = SparseOrbit::new(
        12000.0,
        9000.0,
        0.3,
        Body::new(1.0, 1000.0, 100000.0),
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

    let spline = generate_chi_spline(o.initial, o.body.mu(), Nanotime::secs(500)).unwrap();

    s.bench_function("eval_spline", |b| {
        b.iter(|| {
            let t = black_box(32.5);
            spline.sample(t);
        })
    });

    s.bench_function("eval_lut", |b| {
        lookup_ta_from_ma(0.0, 0.0);
        b.iter(|| {
            let ma: f32 = black_box(PI * 1.2);
            let ecc = black_box(0.32);
            lookup_ta_from_ma(ma, ecc);
        })
    });

    s.finish();

    let mut g = c.benchmark_group("Sim");

    g.sample_size(1000);

    let (mut scenario, _) = stable_simulation();
    let mut t = Nanotime::zero();

    g.bench_function("scenario_sim", |b| {
        b.iter(|| {
            t += Nanotime::secs(10);
            scenario.simulate(t, Nanotime::secs(20));
        })
    });

    g.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
