use crate::assets::load_names_from_file;
use crate::multiplayer::WorldAction;
use crate::multiplayer::apply_world_action;
use crate::sim::systems::*;
use crate::sim::world::*;
use bary_core::prelude::*;
use log::*;
use std::path::PathBuf;

pub struct WorldBuilder {
    assets_dir: Option<String>,
    blueprints: Vec<BlueprintId>,
    spawns: Vec<(BlueprintId, Option<String>, Isometry2d)>,
    commands: Vec<WorldAction>,
}

impl WorldBuilder {
    pub fn new() -> Self {
        Self {
            assets_dir: None,
            blueprints: Vec::new(),
            spawns: Vec::new(),
            commands: Vec::new(),
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

    pub fn blueprint(mut self, id: impl Into<BlueprintId>) -> Self {
        self.blueprints.push(id.into());
        self
    }

    pub fn spawn(
        mut self,
        bp_id: impl Into<BlueprintId>,
        name: impl Into<String>,
        iso: impl Into<Isometry2d>,
    ) -> Self {
        let name = name.into();
        let name = if name.is_empty() { None } else { Some(name) };
        self.spawns.push((bp_id.into(), name, iso.into()));
        self
    }

    pub fn waypoint(mut self, grid_name: &str, waypoint: impl Into<Isometry2d>) -> Self {
        let cmd = WorldAction::SetWaypointByName {
            name: grid_name.to_string(),
            waypoint: waypoint.into(),
        };
        self.commands.push(cmd);
        self
    }

    pub fn command(mut self, action: WorldAction) -> Self {
        self.commands.push(action);
        self
    }

    pub fn build(self) -> World {
        let mut world = World::empty();

        if let Some(assets_dir) = self.assets_dir {
            let parts_dir = PathBuf::from(&assets_dir).join("parts");

            let parts = load_parts_from_dir(&parts_dir).expect("Parts dir");

            let ship_names_path = PathBuf::from(&assets_dir).join("ship_names.txt");
            world.ship_names = load_names_from_file(ship_names_path).unwrap_or(vec![]);

            for (_, part) in &parts {
                let id = world.spawner.spawn();
                world.prototypes.spawn(id, part.clone());
            }

            for bpid in self.blueprints {
                let id = world.spawner.spawn();
                let bp = load_blueprint(&bpid, &assets_dir, &parts).expect("Vehicle dir");
                let bp = NamedBlueprint {
                    id: bpid,
                    blueprint: bp,
                };
                info!(
                    "Loaded blueprint: {} v{} ({} parts)",
                    bp.id.0,
                    bp.id.1,
                    bp.blueprint.part_count()
                );
                world.blueprints.spawn(id, bp);
            }
        }

        for (bp_id, name, iso) in self.spawns {
            let id = match name {
                Some(name) => spawn_grid_with_bp_id(&mut world, &bp_id, &name),
                None => spawn_grid_with_random_name(&mut world, bp_id),
            };

            if let Ok(id) = id {
                _ = set_grid_pose(&mut world, id, iso);
            }
        }

        for action in self.commands {
            apply_world_action(&mut world, action);
        }

        world
    }
}
