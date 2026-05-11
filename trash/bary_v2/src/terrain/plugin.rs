use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::system_sets::DrawSet;
use crate::tick_schedule::SimTick;

use super::debug_systems;
use super::flood_fill;
use super::messages;
use super::systems;
use super::types;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(types::ChunkMap::default());

        // messages
        app.add_message::<messages::GenerateChunk>();
        app.add_message::<messages::DeleteChunk>();
        app.add_message::<messages::Excavate>();
        app.add_message::<messages::MineToInventory>();

        app.add_systems(Startup, systems::insert_tiles);

        // app.add_systems(
        //     SimTick,
        //     (
        //         systems::generate_tiles,
        //         systems::delete_chunks,
        //         systems::excavate_chunks,
        //         systems::process_excavators,
        //         systems::process_mine_to_inventory.pipe(systems::spawn_mining_visuals),
        //         systems::age_mining_visuals_system,
        //     )
        //         .chain(),
        // );

        app.add_systems(FixedUpdate, systems::update_meshes);

        app.add_systems(
            Update,
            (
                systems::render_mining_indicators_system,
                // debug rendering
                debug_systems::draw_excavators.in_set(DrawSet),
                debug_systems::draw_hovered_grid_and_tile.in_set(DrawSet),
                debug_systems::trigger_mouse_digging,
                // flood fill stuff. just a demo for cave detection.
                // not a serious endeavor.
                flood_fill::spawn_flood_fill,
                flood_fill::update_flood_fill,
                flood_fill::draw_flood_fill.in_set(DrawSet),
            ),
        );

        app.add_systems(EguiPrimaryContextPass, debug_systems::debug_ui);
    }
}
