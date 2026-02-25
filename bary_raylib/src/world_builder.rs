use std::path::PathBuf;

use crate::world::World;
use bary_core::prelude::*;

pub struct WorldBuilder {
    assets_dir: Option<String>,
    blueprints: Vec<String>,
}

impl WorldBuilder {
    pub fn new() -> Self {
        Self {
            assets_dir: None,
            blueprints: Vec::new(),
        }
    }

    pub fn assets(mut self, assets_dir: &str) -> Self {
        self.assets_dir = Some(assets_dir.to_string());
        self
    }

    pub fn blueprint(mut self, name: &str) -> Self {
        self.blueprints.push(name.to_string());
        self
    }

    pub fn build(self) -> World {
        let mut world = World::empty();

        if let Some(assets_dir) = self.assets_dir {
            let parts_dir = PathBuf::from(&assets_dir).join("parts");
            let vehicles_dir = PathBuf::from(&assets_dir).join("vehicles");

            let parts = load_parts_from_dir(&parts_dir).expect("Parts dir");

            for (_, part) in &parts {
                let id = world.spawner.spawn();
                world.prototypes.spawn(id, part.clone());
            }

            for v in self.blueprints {
                let id = world.spawner.spawn();
                let path = vehicles_dir.join(format!("{}.vehicle", v));
                let bp = load_vehicle(path, &parts).expect("Vehicle dir");
                world.blueprints.spawn(id, (v.to_string(), bp));
            }
        }

        world
    }
}
