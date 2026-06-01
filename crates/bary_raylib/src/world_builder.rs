use crate::{sim::*, world_builder::WorldBuilderCommand::SetAnchored};
use bary_core::prelude::*;
use bary_factory::*;
use bary_parts::*;
use bary_sim::*;
use log::*;
use std::path::PathBuf;

enum WorldBuilderCommand {
    LoadBlueprint(BlueprintId),
    SpawnShip(BlueprintId, Option<String>, Isometry2d),
    ModifyWorld(WorldDelta),
    InsertDebugSource(PartCoord, Item),
    InsertPipe(PartCoord, PartCoord),
    SetRecipe(PartCoord, RecipeListing),
    SpawnAsteroid(Isometry2d, f32, u64),
    SetAnchored(bool),
}

pub struct WorldBuilder {
    assets_dir: Option<String>,
    commands: Vec<WorldBuilderCommand>,
    cursor_grid: Option<Ent>,
}

impl WorldBuilder {
    pub fn new() -> Self {
        Self {
            assets_dir: None,
            commands: Vec::new(),
            cursor_grid: None,
        }
    }

    pub fn assets(mut self) -> Self {
        self.assets_dir = Some("./assets/".to_string());
        self
    }

    pub fn test_assets(mut self) -> Self {
        self.assets_dir = Some("../../assets/".to_string());
        self
    }

    pub fn blueprint(mut self, id: impl Into<BlueprintId>) -> Self {
        let cmd = WorldBuilderCommand::LoadBlueprint(id.into());
        self.commands.push(cmd);
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
        let cmd = WorldBuilderCommand::SpawnShip(bp_id.into(), name, iso.into());
        self.commands.push(cmd);
        self
    }

    pub fn insert_source(mut self, coord: impl Into<PartCoord>, item: Item) -> Self {
        let cmd = WorldBuilderCommand::InsertDebugSource(coord.into(), item);
        self.commands.push(cmd);
        self
    }

    pub fn set_recipe(mut self, coord: impl Into<PartCoord>, recipe: RecipeListing) -> Self {
        let cmd = WorldBuilderCommand::SetRecipe(coord.into(), recipe);
        self.commands.push(cmd);
        self
    }

    pub fn insert_pipe(mut self, a: impl Into<PartCoord>, b: impl Into<PartCoord>) -> Self {
        let cmd = WorldBuilderCommand::InsertPipe(a.into(), b.into());
        self.commands.push(cmd);
        self
    }

    pub fn waypoint(mut self, grid_name: &str, waypoint: impl Into<Isometry2d>) -> Self {
        let cmd = WorldDelta::SetWaypointByName {
            name: grid_name.to_string(),
            waypoint: waypoint.into(),
        };
        let cmd = WorldBuilderCommand::ModifyWorld(cmd);
        self.commands.push(cmd);
        self
    }

    pub fn command(mut self, action: WorldDelta) -> Self {
        let cmd = WorldBuilderCommand::ModifyWorld(action);
        self.commands.push(cmd);
        self
    }

    pub fn asteroid(mut self, p: impl Into<Isometry2d>, r: f32, seed: u64) -> Self {
        let cmd = WorldBuilderCommand::SpawnAsteroid(p.into(), r, seed);
        self.commands.push(cmd);
        self
    }

    pub fn anchored(mut self, anchored: bool) -> Self {
        let cmd = WorldBuilderCommand::SetAnchored(anchored);
        self.commands.push(cmd);
        self
    }

    pub fn build(mut self) -> World {
        let mut world = World::empty();

        if let Some(assets_dir) = self.assets_dir {
            let parts_dir = PathBuf::from(&assets_dir).join("parts");

            let ship_names_path = PathBuf::from(&assets_dir).join("ship_names.txt");
            world.ship_names = load_names_from_file(ship_names_path).unwrap_or(vec![]);

            let parts = load_parts_from_dir(&parts_dir).expect("Parts dir");

            for (_, part) in &parts {
                let id = world.spawner.spawn();
                world.prototypes.spawn(id, part.clone());
            }

            for cmd in self.commands {
                match cmd {
                    WorldBuilderCommand::LoadBlueprint(bpid) => {
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
                    WorldBuilderCommand::SpawnShip(bp_id, name, iso) => {
                        let id = match name {
                            Some(name) => spawn_grid_with_bp_id(&mut world, &bp_id, &name),
                            None => spawn_grid_with_random_name(&mut world, bp_id),
                        };

                        match id {
                            Ok(id) => {
                                self.cursor_grid = Some(id);
                                _ = set_grid_pose(&mut world, id, iso);
                            }
                            Err(e) => {
                                error!("Failed to spawn grid: {e:?}");
                            }
                        }
                    }
                    WorldBuilderCommand::ModifyWorld(delta) => {
                        _ = apply_delta(&mut world, delta);
                    }
                    WorldBuilderCommand::InsertDebugSource(coord, item) => {
                        if let Some(grid_id) = self.cursor_grid {
                            let delta = WorldDelta::InsertPart {
                                grid_id,
                                coord,
                                rotation: Rotation::East,
                                layer: PartLayer::Plumbing,
                                name: "debug-source".to_string(),
                            };
                            _ = apply_delta(&mut world, delta);
                            let delta = WorldDelta::SetSourceItem {
                                grid_id,
                                coord,
                                item,
                            };
                            _ = apply_delta(&mut world, delta);
                        }
                    }
                    WorldBuilderCommand::InsertPipe(src, dst) => {
                        if let Some(grid_id) = self.cursor_grid {
                            let delta = WorldDelta::InsertPipe { grid_id, src, dst };
                            _ = apply_delta(&mut world, delta);
                        }
                    }
                    WorldBuilderCommand::SetRecipe(coord, recipe) => {
                        if let Some(grid_id) = self.cursor_grid {
                            let delta = WorldDelta::SetRecipe {
                                grid_id,
                                coord,
                                recipe,
                            };
                            _ = apply_delta(&mut world, delta);
                        }
                    }
                    WorldBuilderCommand::SpawnAsteroid(p, r, seed) => {
                        let delta = WorldDelta::SpawnAsteroid {
                            iso: p,
                            radius: r,
                            seed,
                        };
                        _ = apply_delta(&mut world, delta);
                    }
                    WorldBuilderCommand::SetAnchored(anchored) => {
                        if let Some(grid_id) = self.cursor_grid {
                            let delta = WorldDelta::SetAnchored(grid_id, anchored);
                            _ = apply_delta(&mut world, delta);
                        }
                    }
                }
            }
        }

        world
    }
}
