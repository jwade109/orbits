use std::path::PathBuf;

use crate::components::*;
use crate::vehicle_grid::*;
use crate::world::*;
use bary_core::prelude::*;
use raylib::math::Vector2;

pub fn with_vehicle_data_loaded(assets_dir: &str) -> World {
    let mut world = World::empty();

    let parts_dir = PathBuf::from(assets_dir).join("parts");
    let vehicles_dir = PathBuf::from(assets_dir).join("vehicles");

    let parts = load_parts_from_dir(&parts_dir).expect("Parts dir");

    for (_, part) in &parts {
        let id = world.counter.get_id();
        world.prototypes.spawn(id, (part.clone(), None));
    }

    let vehicles = ["pollux", "bellerophon", "remora", "spacestation"];

    for v in vehicles {
        let id = world.counter.get_id();
        let path = vehicles_dir.join(format!("{}.vehicle", v));
        let bp = load_vehicle(path, &parts).expect("Vehicle dir");
        world.blueprints.spawn(id, (v.to_string(), bp));
    }

    world
}

pub fn dev_world(assets_dir: &str) -> BaryResult<World> {
    let mut world = with_vehicle_data_loaded(assets_dir);

    for (name, bp) in world.blueprints.values() {
        let pos = randvec(1.0, 600.0);
        spawn_grid_from_blueprint(
            &mut world.counter,
            &world.prototypes,
            &mut world.grids,
            &mut world.thrusters,
            &mut world.computers,
            pos,
            name.clone(),
            bp,
        )?;
    }

    Ok(world)
}

pub fn test_camera_snapping() -> World {
    let mut world = World::empty();

    for _ in 0..100 {
        let pos = randvec(1.0, 1000.0);
        let time_left = rand(1.0, 6.0);
        world.particles.push(RingParticle { pos, time_left });
    }

    world.target_camera.target = Vector2::new(100.0, 300.0);
    world.snap_camera_to_local_planet = true;
    world
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dev_scenario() {
        let mut world = dev_world("../assets").expect("Expected a valid world");

        for _ in 0..100 {
            update_world(&mut world, (1080.0, 720.0).into());
        }

        assert_eq!(world.grids.len(), 4);
    }

    #[test]
    fn test_the_world() {
        let mut world = test_camera_snapping();

        for _ in 0..100 {
            update_world(&mut world, (1080.0, 720.0).into());
        }

        assert_eq!(world.camera.offset, Vector2::new(540.0, 360.0));
        assert_eq!(
            world.camera.target,
            Vector2 {
                x: 99.997345,
                y: 299.99207
            }
        );
        assert_eq!(world.camera.rotation, -161.56076);
        assert_eq!(world.camera.zoom, 0.9999761);
    }
}
