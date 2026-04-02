use std::collections::{BTreeMap, BTreeSet};

use bary_core::prelude::*;

use crate::{
    client::GridLocation,
    ops::{set_grid_pose, set_primary_computer_state, set_primary_computer_waypoint},
    query::{blueprint_by_name, grid_by_name},
    result::BaryError,
    sim::{
        PartOccupancy, World, destroy_part,
        find::{self, grid_pose},
        get_grid_physical_props_by_id, insert_part, spawn_empty_grid, split_grid_if_necessary,
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
        .spawn("remora", "", Isometry2d::ZERO)
        .waypoint("remora", (100.0, 400.0, 0.1))
        .build();

    let mut w2 = WorldBuilder::new()
        .test_assets()
        .blueprint("remora")
        .spawn("remora", "", Isometry2d::ZERO)
        .waypoint("remora", (100.0, 400.0, 0.1))
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
        .spawn("remora", "ursula", Isometry2d::ZERO)
        .build();

    let grid_id = grid_by_name(&world.grids, "ursula").unwrap();
    let bp = blueprint_by_name(&world.blueprints, "remora")
        .unwrap()
        .clone();

    for (_id, instance) in bp.parts() {
        let mut instance = instance.clone();
        instance.region.shift((20, 20).into());
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

#[test]
fn splitting_vehicle_should_preserve_part_coordinates() {
    let mut world = WorldBuilder::new()
        .test_assets()
        .blueprint("bellerophon")
        .spawn("bellerophon", "kyle", (0.0, 0.0, 0.3))
        .build();

    let grid_id = grid_by_name(&world.grids, "kyle").unwrap();

    assert_eq!(grid_id, Ent(31));

    assert_eq!(world.grids.len(), 1);
    assert_eq!(world.parts.len(), 245);

    let grid = world.grids.try_get(grid_id).unwrap();

    assert_eq!(grid.origin(), (-10.546879, -7.4537697, 0.3).into());
    assert_eq!(grid.particle_location, (0.0, 0.0, 0.3).into());

    let mut gt_part_coords = BTreeMap::new();
    for (part_id, part) in world.parts.iter() {
        let center = grid.origin() * part.region.center_isometry();
        gt_part_coords.insert(*part_id, center);
    }

    assert_world_is_consistent(&world);

    let test_part_coords = |w: &World| {
        let mut new_part_coords = BTreeMap::new();
        for (part_id, part) in w.parts.iter() {
            let grid = w.grids.try_get(part.grid_id).unwrap();
            let center = grid.origin() * part.region.center_isometry();
            new_part_coords.insert(part_id, (part.grid_id, center));
        }
        for (id, (_grid_id, actual)) in new_part_coords {
            let expected = gt_part_coords.get(id).unwrap();
            let delta = expected.translation - actual.translation;

            assert!(
                delta.length() < 0.01,
                "Expected to find part {} at {}, was actually at {}",
                id,
                expected.translation,
                actual.translation,
            );
        }
    };

    // parts' location in the world should never change beyond this point

    let mut parts_to_destroy = BTreeSet::new();
    for y in 0..100 {
        let c = PartCoord::new((32, y));
        let occ = grid.get_parts_at(c).unwrap_or(&PartOccupancy::EMPTY);
        for (_layer, id) in occ.iter() {
            parts_to_destroy.insert(id);
        }
    }

    assert_eq!(parts_to_destroy.len(), 5);

    let new_grid_id = world.spawner.next();

    for part_id in parts_to_destroy {
        let part = world.parts.try_get(part_id).unwrap();
        let proto = world.prototypes.try_get(part.prototype).unwrap();
        println!("Destroying {}: {:?}", part_id, proto.name);
        let result = destroy_part(&mut world, part_id);
        assert!(result.is_ok());

        // test this condition only when the grid is still whole
        if world.grids.len() == 1 {
            test_part_coords(&world);
        }
    }

    assert_world_is_consistent(&world);

    let grid_a = world.grids.try_get(grid_id).unwrap();
    let grid_b = world.grids.try_get(new_grid_id).unwrap();

    assert_eq!(grid_a.parts_mass, Mass::grams(112097000));
    assert_eq!(grid_b.parts_mass, Mass::grams(17231000));

    assert_eq!(grid_a.parts.len(), 141);
    assert_eq!(grid_b.parts.len(), 99);

    assert_eq!(world.grids.len(), 2);

    test_part_coords(&world);
}

#[test]
fn get_inventory_at_grid_location() {
    let mut world = World::empty();

    let grid_id = spawn_empty_grid(&mut world, "whatever");

    let slots = vec![
        SlotData {
            min: IVec2::ZERO,
            max: IVec2::ONE,
            ..SlotData::default()
        },
        SlotData {
            min: IVec2::ONE,
            max: IVec2::splat(2),
            ..SlotData::default()
        },
    ];

    let inv_data = InventoryData { slots };

    let proto = PartPrototype {
        name: "test-cargo".to_string(),
        mass: Mass::grams(1000),
        dims: UVec2::new(6, 4),
        layer: PartLayer::Internal,
        excavator_data: None,
        computer_data: None,
        inventory_data: Some(inv_data),
        thruster_data: None,
        machine_data: None,
        docking_port_data: None,
    };

    let proto_id = world.spawner.spawn();
    world.prototypes.spawn(proto_id, proto.clone());

    let pl = GridRegion::new((-3, 2), Rotation::North, proto.dims);
    let instance = PartInstance::new("test-cargo", PartLayer::Internal, pl);
    let part_id = insert_part(grid_id, &mut world, &instance, true).unwrap();

    for part in world.parts.values() {
        println!("- {:?}", part);
    }

    for inv in world.inventories.values() {
        for slot in inv.slots() {
            println!("- {:?}", slot);
        }
    }

    let grid = world.grids.try_get(grid_id).unwrap();

    for x in 0..3 {
        for y in 0..6 {
            let occ = grid.get_parts_at((x, y).into());
            assert!(occ.is_some(), "{}, {}", x, y);
        }
    }

    assert_eq!(grid.bounds, ((0, 0).into(), (4, 6).into()));

    // the part looks like this in the grid frame, with inventories
    // indicated by @, #, %
    //
    //    --%%%%--
    //    --%%%%--
    //    ----@@--
    //    ----@@--
    //    ------##
    //    ------##

    let mut loc = GridLocation {
        grid_id,
        coord: PartCoord::ZERO,
    };

    let inv_slot =
        find::inventory_at_c(loc, &world.grids, &world.parts, &world.inventories).unwrap();

    assert_eq!(inv_slot, (part_id, 0));

    loc.coord = (4, 7).into();

    let failure = find::inventory_at_c(loc, &world.grids, &world.parts, &world.inventories);

    assert_eq!(failure, Err(BaryError::NoPartsAt((4, 7).into())));
}
