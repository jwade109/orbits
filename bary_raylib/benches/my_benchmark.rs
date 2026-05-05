use bary_raylib::world_builder::WorldBuilder;
use bary_raylib::{constants::TICKS_PER_SECOND, sim::*};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn run_for_one_hour(world: &mut World) {
    let ticks = TICKS_PER_SECOND * 3600;
    for _ in 0..ticks {
        update_world(world);
    }
}

fn run_inventory_for_one_hour() {
    let ticks = TICKS_PER_SECOND * 3600;
    let mut grid = GridVentory::random(100);
    for _ in 0..ticks {
        update_inventory(&mut grid);
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut world = WorldBuilder::new()
        .test_assets()
        .blueprint("pollux")
        .spawn("pollux", "polly", (0.0, 0.0, 0.0))
        .waypoint("polly", (100.0, 200.0, 0.0))
        .build();

    c.bench_function("world_update_pollux", |b| {
        b.iter(|| black_box(update_world(&mut world)))
    });

    c.bench_function("run_for_one_hour", |b| {
        b.iter(|| black_box(run_for_one_hour(&mut world)))
    });

    c.bench_function("run_inventory_for_one_hour", |b| {
        b.iter(|| black_box(run_inventory_for_one_hour()))
    });

    let mut world = WorldBuilder::new()
        .test_assets()
        .blueprint("bellerophon")
        .spawn("bellerophon", "billy", (0.0, 0.0, 0.0))
        .waypoint("billy", (100.0, 200.0, 0.0))
        .build();

    c.bench_function("world_update_bellerophon", |b| {
        b.iter(|| black_box(update_world(&mut world)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
