use std::path::PathBuf;

use crate::systems::*;
use crate::world::*;
use bary_core::prelude::*;

pub struct WorldBuilder {
    assets_dir: Option<String>,
    blueprints: Vec<String>,
    spawns: Vec<(String, Isometry2d)>,
}

impl WorldBuilder {
    pub fn new() -> Self {
        Self {
            assets_dir: None,
            blueprints: Vec::new(),
            spawns: Vec::new(),
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

    pub fn spawn(mut self, name: &str, iso: impl Into<Isometry2d>) -> Self {
        self.spawns.push((name.to_string(), iso.into()));
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

        for (name, iso) in self.spawns {
            if let Ok(id) = world::spawn_grid_by_name(&mut world, &name) {
                _ = world::set_grid_isometry(&mut world, id, iso);
            }
        }

        world
    }
}
