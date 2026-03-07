use crate::{systems::get_sum_part_masses, world::World};

pub fn computer_bidirectional_pointers_are_consistent(world: &World) {
    for (grid_id, grid) in world.grids.iter() {
        for cpu_id in &grid.computers {
            // computer component should exist
            let cpu = world
                .computers
                .try_get(*cpu_id)
                .expect("Expected a computer");

            // computer should point to its parent grid
            assert_eq!(cpu.grid_id, *grid_id);
        }
    }
}

pub fn mass_of_grids_is_accurate(world: &World) {
    for (grid_id, grid) in world.grids.iter() {
        let calc = get_sum_part_masses(world, *grid_id).unwrap();
        let stored = grid.parts_mass;
        assert_eq!(stored, calc);
    }
}

pub fn assert_world_is_consistent(world: &World) {
    mass_of_grids_is_accurate(world);
    computer_bidirectional_pointers_are_consistent(world);
}
