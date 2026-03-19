use bary_core::prelude::{Ent, Isometry2d, Mass, Vec2};

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
fn ship_assembly() {
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

    for _ in 0..3000 {
        for _ in 0..100 {
            update_world(&mut world);
        }

        let pa = grid_pose(&world.grids, grid_id).unwrap();
        let pb = grid_pose(&world.grids, next_id).unwrap();

        println!("{} {:?} {:?}", world.ticks, pa.to_tuple(), pb.to_tuple());
    }
}
