use crate::sim::*;
use anyhow::ensure;
use bary_sim::World;

pub fn computer_pointers_are_consistent(world: &World) -> Result<(), anyhow::Error> {
    for (_grid_id, grid) in world.grids.iter() {
        for id in &grid.computers {
            ensure!(world.computers.try_get(*id).is_ok());
        }
    }

    Ok(())
}

pub fn part_bidirectional_pointers_are_consistent(world: &World) -> Result<(), anyhow::Error> {
    for (grid_id, grid) in world.grids.iter() {
        for id in &grid.parts {
            let part = world.parts.try_get(*id).unwrap();
            ensure!(part.grid_id == *grid_id);
        }
    }

    for (id, part) in world.parts.iter() {
        let grid = world.grids.try_get(part.grid_id).unwrap();
        ensure!(grid.parts.contains(id));
    }

    Ok(())
}

pub fn mass_of_grids_is_accurate(world: &World) -> Result<(), anyhow::Error> {
    for (grid_id, grid) in world.grids.iter() {
        let expected = get_grid_physical_props_by_id(*grid_id, &world.grids, &world.parts).unwrap();
        let actual = (grid.parts_mass, grid.center_of_mass);
        ensure!(actual.0 == expected.0);
        ensure!(actual.1 == expected.1);
    }

    Ok(())
}

pub fn grid_parts_do_not_intersect(world: &World) -> Result<(), anyhow::Error> {
    for grid in world.grids.values() {
        let mut expected_cells = 0;
        for part_id in &grid.parts {
            let part = world.parts.try_get(*part_id).unwrap();
            let n_cells = part.region.cell_count();
            expected_cells += n_cells;
        }

        let mut actual_cells = 0;
        for (_idx, occ) in &grid.occupancy {
            actual_cells += occ.iter().count() as u32;
        }

        ensure!(expected_cells == actual_cells);
    }

    Ok(())
}

pub fn is_world_consistent(world: &World) -> Result<(), anyhow::Error> {
    mass_of_grids_is_accurate(world)?;
    computer_pointers_are_consistent(world)?;
    part_bidirectional_pointers_are_consistent(world)?;
    grid_parts_do_not_intersect(world)?;

    Ok(())
}

pub fn assert_world_is_consistent(world: &World) {
    match is_world_consistent(world) {
        Ok(_) => (),
        Err(e) => {
            println!("Failed: {:?}", e);
            assert!(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{sim::update_world, world_builder::WorldBuilder};
    use bary_core::prelude::*;
    use bary_sim::{load_world, save_world};

    #[test]
    fn world_persistence() {
        let save_path = "../../saves/test_world";

        if std::fs::exists(save_path).unwrap() {
            std::fs::remove_dir_all(save_path).unwrap();
        }

        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("remora")
            .blueprint("bellerophon")
            .blueprint("foundation")
            .spawn("pollux", "billy", (120.0, 43.0, 0.4))
            .spawn("remora", "sally", (-30.0, 21.0, -0.1))
            .spawn("bellerophon", "eisenhower", (50.0, 109.0, 1.4))
            .waypoint("pollux", (50.0, 300.0, 0.2))
            .build();

        for _ in 0..1000 {
            update_world(&mut world);
        }

        let r = save_world(save_path, &world, false);
        assert_eq!(r, Ok(()));

        let r = save_world(save_path, &world, false);
        assert_eq!(r, Err(BaryError::SaveAlreadyExists));

        let r = save_world(save_path, &world, true);
        assert_eq!(r, Ok(()));

        let world = load_world(save_path).expect("Expected successful load");

        assert_eq!(world.grids.len(), 3);
        assert_eq!(world.parts.len(), 374);
    }
}
