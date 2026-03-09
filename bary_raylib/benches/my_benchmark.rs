use bary_raylib::systems::*;
use bary_raylib::world::*;
use bary_raylib::world_builder::WorldBuilder;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn run_for_one_hour(world: &mut World) {
    let ticks = TICKS_PER_SECOND * 3600;
    for _ in 0..ticks {
        update_world(world);
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut world = WorldBuilder::new()
        .test_assets()
        .blueprint("pollux")
        .spawn("pollux", (0.0, 0.0, 0.0))
        .waypoint("pollux", (100.0, 200.0, 0.0))
        .build();

    c.bench_function("world_update", |b| {
        b.iter(|| black_box(update_world(&mut world)))
    });

    c.bench_function("run_for_one_hour", |b| {
        b.iter(|| black_box(run_for_one_hour(&mut world)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
