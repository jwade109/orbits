use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use starling::core::Nanotime;
use starling::examples::make_earth;

fn criterion_benchmark(c: &mut Criterion) {
    let o = SparseOrbit::from_pv(((500.0, 200.0), (-12.0, 30.0)), make_earth(), Nanotime(0)).unwrap();

    c.bench_function("pv_at_time", |b| {
        b.iter(|| {
            let t = black_box(Nanotime::secs_f32(32.5));
            o.pv_at_time(t);
        })
    });

    let spline = generate_chi_spline(
        o.initial,
        o.body.mu(),
        o.period().unwrap_or(Nanotime::secs(500)),
    )
    .unwrap();

    c.bench_function("pv_at_time_faster", |b| {
        b.iter(|| {
            let t = black_box(Nanotime::secs_f32(3.0));
            o.pv_at_time_spline(t, &spline)
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
