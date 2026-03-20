use bary_core::prelude::*;

use crate::{
    ops::{set_grid_pose, set_primary_computer_state, set_primary_computer_waypoint},
    query::{blueprint_by_name, grid_by_name},
    sim::{
        find::grid_pose, get_grid_physical_props_by_id, insert_part, split_grid_if_necessary,
        update_world,
    },
    tests::assert_world_is_consistent,
    world_builder::WorldBuilder,
};

#[test]
fn vehicle_pathing_is_deterministic() {
    let mut w1 = WorldBuilder::new()
        .test_assets()
        .blueprint("remora")
        .spawn("remora", Isometry2d::ZERO)
        // .waypoint("remora", (100.0, 400.0, 0.1))
        .commands("remora")
        .build();

    let mut w2 = WorldBuilder::new()
        .test_assets()
        .blueprint("remora")
        .spawn("remora", Isometry2d::ZERO)
        // .waypoint("remora", (100.0, 400.0, 0.1))
        .commands("remora")
        .build();

    for _ in 0..10000 {
        update_world(&mut w1);
        update_world(&mut w2);

        assert_eq!(w1.ticks, w2.ticks);

        for (g1, g2) in w1.grids.values().zip(w2.grids.values()) {
            assert_eq!(
                g1.particle_location, g2.particle_location,
                "Failed on tick {}",
                w1.ticks
            );
        }
    }
}

#[test]
fn build_ship_on_another_ship_then_navigate() {
    let mut world = WorldBuilder::new()
        .test_assets()
        .blueprint("remora")
        .spawn("remora", Isometry2d::ZERO)
        .build();

    let grid_id = grid_by_name(&world.grids, "remora").unwrap();
    let bp = blueprint_by_name(&world.blueprints, "remora")
        .unwrap()
        .clone();

    for (_id, instance) in bp.parts() {
        let mut instance = instance.clone();
        instance.placement.shift((20, 20).into());
        assert!(insert_part(grid_id, &mut world, &instance, true).is_ok());
    }

    let next_id = world.spawner.next();

    assert_eq!(world.grids.len(), 1);

    let grids = split_grid_if_necessary(&mut world, grid_id).unwrap();

    assert_eq!(world.grids.len(), 2);

    assert_eq!(grids, vec![grid_id, next_id]);

    let props_a = get_grid_physical_props_by_id(grid_id, &world.grids, &world.parts).unwrap();
    let props_b = get_grid_physical_props_by_id(next_id, &world.grids, &world.parts).unwrap();

    assert_eq!(props_a.0, Mass::grams(12339000));
    assert_eq!(props_a.1, Vec2::new(2.4357123, 1.2572942));
    assert_eq!(props_a, props_b);

    let grid_a = world.grids.try_get(grid_id).unwrap();
    let grid_b = world.grids.try_get(next_id).unwrap();

    assert_eq!(grid_a.bounds, grid_b.bounds);

    _ = set_grid_pose(&mut world, next_id, (-10.0, -20.0, 0.3).into());

    assert_world_is_consistent(&world);

    for id in [grid_id, next_id] {
        assert!(set_primary_computer_waypoint(id, (500.0, 800.0, 0.3), &mut world).is_ok());
        assert!(set_primary_computer_state(id, true, &mut world).is_ok());
    }

    for _ in 0..20 {
        for _ in 0..1000 {
            update_world(&mut world);
        }

        let pa = grid_pose(&world.grids, grid_id).unwrap();
        let pb = grid_pose(&world.grids, next_id).unwrap();

        let a1 = Angle::radians(pa.rotation);
        let a2 = Angle::radians(pb.rotation);

        println!(
            "{} {:?} {:?} {a1} {a2}",
            world.ticks,
            pa.to_tuple(),
            pb.to_tuple()
        );
    }

    let pa = grid_pose(&world.grids, grid_id).unwrap();
    let pb = grid_pose(&world.grids, next_id).unwrap();

    assert_eq!(pa.to_tuple(), (499.92654, 799.72156, -5.9987755));
    assert_eq!(pb.to_tuple(), (499.41153, 799.95984, -5.9996347));
}
