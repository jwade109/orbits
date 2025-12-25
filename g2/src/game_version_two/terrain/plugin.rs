use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

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
        app.add_event::<messages::GenerateChunk>();
        app.add_event::<messages::DeleteChunk>();
        app.add_event::<messages::Excavate>();

        app.add_systems(Startup, systems::insert_tiles);

        app.add_systems(
            Update,
            (
                systems::update_meshes,
                systems::generate_tiles,
                systems::delete_chunks,
                systems::excavate_chunks,
                systems::process_excavators,
                // debug rendering
                debug_systems::draw_excavators,
                debug_systems::draw_hovered_grid_and_tile,
                debug_systems::draw_highlighted_lattice_points,
                // flood fill stuff. just a demo for cave detection.
                // not a serious endeavor.
                flood_fill::spawn_flood_fill,
                flood_fill::update_flood_fill,
                flood_fill::draw_flood_fill,
            )
                .chain(),
        );

        app.add_systems(EguiPrimaryContextPass, debug_systems::debug_ui);
    }
}
