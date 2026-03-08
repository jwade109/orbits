use crate::{query::sum_part_masses_w, world::World};

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

pub fn light_bidirectional_pointers_are_consistent(world: &World) {
    for (grid_id, grid) in world.grids.iter() {
        for id in &grid.lights {
            let light = world.lights.try_get(*id).unwrap();
            assert_eq!(light.grid_id, *grid_id);
        }
    }

    for (id, light) in world.lights.iter() {
        let grid = world.grids.try_get(light.grid_id).unwrap();
        assert!(grid.lights.contains(id));
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
        let calc = sum_part_masses_w(world, *grid_id).unwrap();
        let stored = grid.parts_mass;
        assert_eq!(stored, calc);
    }
}

pub fn assert_world_is_consistent(world: &World) {
    mass_of_grids_is_accurate(world);
    computer_bidirectional_pointers_are_consistent(world);
    light_bidirectional_pointers_are_consistent(world);
    part_bidirectional_pointers_are_consistent(world);
}
