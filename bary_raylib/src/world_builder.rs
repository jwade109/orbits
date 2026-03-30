use crate::assets::load_names_from_file;
use crate::sim::systems::*;
use crate::sim::world::*;
use bary_core::prelude::*;
use std::path::PathBuf;

pub struct WorldBuilder {
    assets_dir: Option<String>,
    blueprints: Vec<String>,
    spawns: Vec<(String, Option<String>, Isometry2d)>,
    waypoints: Vec<(String, Isometry2d)>,
}

impl WorldBuilder {
    pub fn new() -> Self {
        Self {
            assets_dir: None,
            blueprints: Vec::new(),
            spawns: Vec::new(),
            waypoints: Vec::new(),
        }
    }

    pub fn assets(mut self) -> Self {
        self.assets_dir = Some("./assets/".to_string());
        self
    }

    pub fn test_assets(mut self) -> Self {
        self.assets_dir = Some("../assets/".to_string());
        self
    }

    pub fn blueprint(mut self, name: &str) -> Self {
        self.blueprints.push(name.to_string());
        self
    }

    pub fn spawn(
        mut self,
        bp_name: &str,
        name: impl Into<String>,
        iso: impl Into<Isometry2d>,
    ) -> Self {
        let name = name.into();
        let name = if name.is_empty() { None } else { Some(name) };
        self.spawns.push((bp_name.to_string(), name, iso.into()));
        self
    }

    pub fn waypoint(mut self, grid_name: &str, waypoint: impl Into<Isometry2d>) -> Self {
        self.waypoints
            .push((grid_name.to_string(), waypoint.into()));
        self
    }

    pub fn build(self) -> World {
        let mut world = World::empty();

        if let Some(assets_dir) = self.assets_dir {
            let parts_dir = PathBuf::from(&assets_dir).join("parts");
            let vehicles_dir = PathBuf::from(&assets_dir).join("vehicles");
            let parts = load_parts_from_dir(&parts_dir).expect("Parts dir");

            let ship_names_path = PathBuf::from(&assets_dir).join("ship_names.txt");
            world.ship_names = load_names_from_file(ship_names_path).unwrap_or(vec![]);

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

        for (bp_name, name, iso) in self.spawns {
            let id = match name {
                Some(name) => spawn_grid_by_name(&mut world, &bp_name, &name),
                None => spawn_grid_with_random_name(&mut world, &bp_name),
            };

            if let Ok(id) = id {
                _ = set_grid_pose(&mut world, id, iso);
            }
        }

        for (name, waypoint) in self.waypoints {
            if let Some(grid_id) = find::grid_by_name(&world.grids, &name) {
                _ = set_primary_computer_waypoint(grid_id, waypoint, &mut world);
                _ = set_primary_computer_state(grid_id, true, &mut world);
                _ = toggle_tracking(&mut world, grid_id);
            } else {
                log::warn!("Failed to find grid with name {name}");
            }
        }

        world
    }
}
