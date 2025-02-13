use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use starling::core::Nanotime;
use starling::examples::make_earth;
use starling::orbits::sparse_orbit::SparseOrbit;
use starling::orbits::universal::generate_chi_spline;

fn criterion_benchmark(c: &mut Criterion) {
    let o =
        SparseOrbit::from_pv(((500.0, 200.0), (-12.0, 30.0)), make_earth(), Nanotime(0)).unwrap();

    c.bench_function("pv_at_time", |b| {
        b.iter(|| {
            let t = black_box(Nanotime::secs_f32(32.5));
            o.pv_at_time(t);
        })
    });

    c.bench_function("position_at", |b| {
        b.iter(|| {
            let ta = black_box(0.43);
            o.position_at(ta);
        })
    });

    let spline = generate_chi_spline(o.initial, o.body.mu(), Nanotime::secs(500)).unwrap();

    c.bench_function("eval_spline", |b| {
        b.iter(|| {
            let t = black_box(32.5);
            spline.sample(t);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
