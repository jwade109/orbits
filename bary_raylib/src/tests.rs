use crate::world::*;
use bary_core::prelude::*;
use raylib::math::Vector2;

pub fn test_camera_snapping() -> World {
    let mut world = World::empty();

    for i in 0..100 {
        let pos = randvec(1.0, 1000.0);
        let age_left = rand(1.0, 6.0);
        let e1 = EntityId(i);
        let e2 = world.ring_particles.spawn(RingParticle { pos, age_left });
        assert_eq!(e1, e2);
    }

    world.target_camera.target = Vector2::new(100.0, 300.0);
    world.snap_camera_to_local_planet = true;
    world
}

#[cfg(test)]
mod tests {
    use super::*;

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
