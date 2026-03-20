use std::path::PathBuf;

use crate::sim::systems::*;
use crate::sim::world::*;
use bary_core::prelude::*;

pub struct WorldBuilder {
    assets_dir: Option<String>,
    blueprints: Vec<String>,
    spawns: Vec<(String, Isometry2d)>,
    waypoints: Vec<(String, Isometry2d)>,
    with_commands: Vec<String>,
}

impl WorldBuilder {
    pub fn new() -> Self {
        Self {
            assets_dir: None,
            blueprints: Vec::new(),
            spawns: Vec::new(),
            waypoints: Vec::new(),
            with_commands: Vec::new(),
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

    pub fn spawn(mut self, name: &str, iso: impl Into<Isometry2d>) -> Self {
        self.spawns.push((name.to_string(), iso.into()));
        self
    }

    pub fn commands(mut self, name: &str) -> Self {
        self.with_commands.push(name.to_string());
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
                _ = world::set_grid_pose(&mut world, id, iso);
            }
        }

        for (name, waypoint) in self.waypoints {
            if let Some(grid_id) = find::grid_by_name(&world.grids, &name) {
                _ = world::set_primary_computer_waypoint(grid_id, waypoint, &mut world);
                _ = world::set_primary_computer_state(grid_id, true, &mut world);
                _ = world::toggle_tracking(&mut world, grid_id);
            }
        }

        for name in self.with_commands {
            if let Some(grid_id) = find::grid_by_name(&world.grids, &name) {
                _ = world::enqueue_commands_on_primary_computer(grid_id, &mut world);
                _ = world::set_primary_computer_state(grid_id, true, &mut world);
            }
        }

        world
    }
}
