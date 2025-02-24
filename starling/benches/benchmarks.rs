use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use starling::examples::make_earth;
use starling::orbits::generate_chi_spline;
use starling::prelude::*;

fn criterion_benchmark(c: &mut Criterion) {
    let mut s = c.benchmark_group("Small");

    s.measurement_time(std::time::Duration::from_secs(1));

    let o =
        SparseOrbit::from_pv(((500.0, 200.0), (-12.0, 30.0)), make_earth(), Nanotime::zero()).unwrap();

    s.bench_function("pv_at_time", |b| {
        b.iter(|| {
            let t = black_box(Nanotime::secs_f32(32.5));
            o.pv_at_time(t);
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
