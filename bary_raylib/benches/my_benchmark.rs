use bary_raylib::systems::*;
use bary_raylib::world::*;
use bary_raylib::world_builder::WorldBuilder;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let mut world = WorldBuilder::new()
        .test_assets()
        .blueprint("pollux")
        .blueprint("bellerophon")
        .blueprint("remora")
        .blueprint("spacestation")
        .build();

    _ = world::spawn_grid_by_name(&mut world, "pollux");
    _ = world::spawn_grid_by_name(&mut world, "bellerophon");
    _ = world::spawn_grid_by_name(&mut world, "remora");
    _ = world::spawn_grid_by_name(&mut world, "spacestation");

    assert_eq!(world.grids.len(), 4);

    c.bench_function("world_update", |b| {
        b.iter(|| black_box(update_world(&mut world)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
