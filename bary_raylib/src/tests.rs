#[cfg(test)]
mod tests {
    use crate::world::*;
    use raylib::prelude::*;

    #[test]
    fn test_the_world() {
        let (mut rl, thread) = raylib::init().log_level(TraceLogLevel::LOG_WARNING).build();
        let texture = rl
            .load_texture(&thread, "../assets/parts/cargo/skin.png")
            .unwrap();
        let mut world = World::test_scene(texture);

        for _ in 0..100 {
            update_world(&mut world);
        }

        dbg!(&world);
    }
}
