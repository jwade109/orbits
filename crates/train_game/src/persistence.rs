use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::world::World;

fn load<T: for<'a> Deserialize<'a>>(dir: impl AsRef<Path>, name: &str) -> Option<T> {
    let dir = dir.as_ref();
    let s = std::fs::read(dir.join(name)).ok()?;
    let comp = bincode::deserialize(&s).ok()?;
    Some(comp)
}

fn save<T: Serialize>(val: &T, dir: impl AsRef<Path>, name: &str) -> Option<()> {
    let dir = dir.as_ref();
    let s = bincode::serialize(val).ok()?;
    std::fs::write(dir.join(name), s).ok()?;
    Some(())
}

pub fn save_world(world: &World, path: impl AsRef<Path>) -> Option<()> {
    let path = path.as_ref();
    _ = std::fs::create_dir(path);

    save(&world.spawner, path, "spawner")?;
    save(&world.segments, path, "segments")?;
    save(&world.nodes, path, "nodes")?;
    Some(())
}

pub fn load_world(world: &mut World, path: impl AsRef<Path>) -> Option<()> {
    let path = path.as_ref();

    let spawner = load(path, "spawner")?;
    let segments = load(path, "segments")?;
    let nodes = load(path, "nodes")?;

    world.spawner = spawner;
    world.segments = segments;
    world.nodes = nodes;

    Some(())
}
