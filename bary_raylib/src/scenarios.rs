use crate::result::BaryResult;
use crate::systems::*;
use crate::world::*;
use crate::world_builder::WorldBuilder;
use bary_core::prelude::*;
use std::path::PathBuf;

pub fn with_vehicle_data_loaded(assets_dir: &str, vehicles: &[&str]) -> World {
    let mut world = World::empty();

    let parts_dir = PathBuf::from(assets_dir).join("parts");
    let vehicles_dir = PathBuf::from(assets_dir).join("vehicles");

    let parts = load_parts_from_dir(&parts_dir).expect("Parts dir");

    for (_, part) in &parts {
        let id = world.spawner.spawn();
        world.prototypes.spawn(id, part.clone());
    }

    for v in vehicles {
        let id = world.spawner.spawn();
        let path = vehicles_dir.join(format!("{}.vehicle", v));
        let bp = load_vehicle(path, &parts).expect("Vehicle dir");
        world.blueprints.spawn(id, (v.to_string(), bp));
    }

    world
}

pub fn dev_world(assets_dir: &str) -> BaryResult<World> {
    let mut world = WorldBuilder::new()
        .assets(assets_dir)
        .blueprint("pollux")
        .blueprint("bellerophon")
        .blueprint("remora")
        .blueprint("spacestation")
        .build();

    let bps = world.blueprints.clone();

    for (name, bp) in bps.values() {
        let id = world::spawn_grid_from_blueprint(&mut world, name.clone(), bp)?;
        let grid = world.grids.try_get_mut(id)?;
        grid.isometry.translation = randvec(10.0, 100.0);
        grid.isometry.rotation = rand(-0.3, 0.3);
        grid.angular_velocity = rand(-0.1, 0.1);
        grid.linear_velocity = randvec(0.1, 1.0);
    }

    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;
    use raylib::math::Vector2;

    #[test]
    fn snap_camera_to_local_planet() {
        let mut world = World::empty();

        world.target_camera.target = Vector2::new(100.0, 300.0);
        world.snap_camera_to_local_planet = true;

        for _ in 0..100 {
            update_world(&mut world);
        }

        assert_eq!(world.camera.offset, Vector2::new(540.0, 360.0));
        assert_eq!(
            world.camera.target,
            Vector2 {
                x: 99.999985,
                y: 299.99994
            }
        );
        assert_eq!(world.camera.rotation, -161.56502);
        assert_eq!(world.camera.zoom, 7.999999);
    }
}
