use crate::world::World;
use crate::{ops, query};

pub fn computer_bidirectional_pointers_are_consistent(world: &World) {
    for (grid_id, grid) in world.grids.iter() {
        for id in &grid.computers {
            let cpu = world.computers.try_get(*id).unwrap();
            assert_eq!(cpu.grid_id, *grid_id);
        }
    }

    for (id, cpu) in world.computers.iter() {
        let grid = world.grids.try_get(cpu.grid_id).unwrap();
        assert!(grid.computers.contains(id));
    }
}

pub fn part_bidirectional_pointers_are_consistent(world: &World) {
    for (grid_id, grid) in world.grids.iter() {
        for id in &grid.parts {
            let part = world.parts.try_get(*id).unwrap();
            assert_eq!(part.grid_id, *grid_id);
        }
    }

    for (id, part) in world.parts.iter() {
        let grid = world.grids.try_get(part.grid_id).unwrap();
        assert!(grid.parts.contains(id));
    }
}

pub fn mass_of_grids_is_accurate(world: &World) {
    for (grid_id, grid) in world.grids.iter() {
        let expected =
            query::get_grid_physical_props_by_id(*grid_id, &world.grids, &world.parts).unwrap();
        let actual = (grid.parts_mass, grid.center_of_mass);
        assert_eq!(actual.0, expected.0);
        assert_eq!(
            actual.1, expected.1,
            "Expected is {}, actual is {}",
            expected.1, actual.1
        );
    }
}

pub fn assert_world_is_consistent(world: &World) {
    mass_of_grids_is_accurate(world);
    computer_bidirectional_pointers_are_consistent(world);
    part_bidirectional_pointers_are_consistent(world);
}
