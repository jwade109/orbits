use bevy::prelude::*;
use bevy_ecs::system::SystemParam;

use super::types::*;
use super::utils::*;

#[derive(SystemParam)]
pub struct TerrainHelper<'w, 's> {
    grid: Res<'w, ChunkMap>,
    chunks: Query<'w, 's, &'static TerrainChunk>,
}

impl TerrainHelper<'_, '_> {
    pub fn chunk_at(&self, pos: Vec2) -> Option<&TerrainChunk> {
        let g = to_grid(pos);
        let e = self.grid.get(&g)?;
        let chunk = self.chunks.get(*e).ok()?;
        Some(chunk)
    }

    pub fn tile_at(&self, pos: Vec2) -> Option<&Tile> {
        let (_, l) = to_grid_and_lattice(pos);
        let chunk = self.chunk_at(pos)?;
        let dense = chunk.dense.as_ref()?;
        let tile = &dense.points[l.x as usize][l.y as usize];
        Some(tile)
    }
}
