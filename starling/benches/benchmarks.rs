use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use starling::examples::make_earth;
use starling::nanotime::Nanotime;
use starling::orbits::generate_chi_spline;
use starling::orbits::SparseOrbit;
use starling::math::bhaskara_sin_approx;

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

    c.bench_function("sine", |b| {
        b.iter(|| {
            let t: f32 = black_box(0.32);
            _ = t.sin();
        })
    });

    c.bench_function("sine_approx", |b| {
        b.iter(|| {
            let t = black_box(0.32);
            _ = bhaskara_sin_approx(t);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
