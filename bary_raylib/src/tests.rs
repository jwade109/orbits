#[cfg(test)]
mod tests {
    use bary_core::prelude::*;
    use raylib::math::Vector2;

    use crate::world::*;

    #[test]
    fn test_the_world() {
        let mut world = World::empty();

        for i in 0..100 {
            let pos = randvec(1.0, 1000.0);
            let age_left = rand(1.0, 6.0);
            let e1 = EntityId(i);
            let e2 = world.ring_particles.spawn(RingParticle { pos, age_left });
            assert_eq!(e1, e2);
        }

        for _ in 0..100 {
            update_world(&mut world, (1080.0, 720.0).into());
        }

        assert_eq!(world.camera.offset, Vector2::new(539.9857, 359.99045));
    }
}
