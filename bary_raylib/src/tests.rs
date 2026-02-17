#[cfg(test)]
mod tests {
    use crate::world::*;

    #[test]
    fn test_the_world() {
        let mut world = World::test_scene();

        for _ in 0..100 {
            update_world(&mut world);
        }

        dbg!(&world);
    }
}
