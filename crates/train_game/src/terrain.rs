use std::collections::BTreeSet;

use bary_core::prelude::{Ent, Isometry2d, vfloor_f64};
use glam::{DVec2, IVec2};
use rend::Color;

use crate::world::World;

pub const TERRAIN_CHUNK_WIDTH_METERS: f64 = 500.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkIndex(IVec2);

impl PartialOrd for ChunkIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let a = (self.0.x, self.0.y);
        let b = (other.0.x, other.0.y);
        a.partial_cmp(&b)
    }
}

impl Ord for ChunkIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let a = (self.0.x, self.0.y);
        let b = (other.0.x, other.0.y);
        a.cmp(&b)
    }
}

impl From<IVec2> for ChunkIndex {
    fn from(value: IVec2) -> Self {
        Self(value)
    }
}

impl ChunkIndex {
    pub fn new(index: impl Into<IVec2>) -> Self {
        Self(index.into())
    }

    pub fn isometry(&self) -> Isometry2d {
        let pos = TERRAIN_CHUNK_WIDTH_METERS * self.0.as_dvec2();
        pos.into()
    }

    pub fn as_ivec2(&self) -> IVec2 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct TerrainChunk {
    index: ChunkIndex,
    tracks: BTreeSet<Ent>,
    nodes: BTreeSet<Ent>,
    color: Color,
    height: [f32; 4],
}

impl TerrainChunk {
    pub fn new(index: impl Into<ChunkIndex>, color: Color) -> Self {
        use bary_core::prelude::rand;

        Self {
            index: index.into(),
            tracks: BTreeSet::new(),
            nodes: BTreeSet::new(),
            color,
            height: [
                rand(0.0, 1.0),
                rand(0.0, 1.0),
                rand(0.0, 1.0),
                rand(0.0, 1.0),
            ],
        }
    }

    pub fn index(&self) -> ChunkIndex {
        self.index
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn add_track(&mut self, track_id: Ent) {
        self.tracks.insert(track_id);
    }

    pub fn add_node(&mut self, node_id: Ent) {
        self.nodes.insert(node_id);
    }

    pub fn remove_track(&mut self, track_id: Ent) {
        self.tracks.remove(&track_id);
    }

    pub fn remove_node(&mut self, node_id: Ent) {
        self.nodes.remove(&node_id);
    }

    pub fn nodes(&self) -> &BTreeSet<Ent> {
        &self.nodes
    }

    pub fn tracks(&self) -> &BTreeSet<Ent> {
        &self.tracks
    }

    pub fn isometry(&self) -> Isometry2d {
        self.index.isometry()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.tracks.is_empty()
    }

    pub fn height(&self) -> [f32; 4] {
        self.height
    }
}

pub fn get_chunk_index(pos: impl Into<DVec2>) -> ChunkIndex {
    ChunkIndex::new(vfloor_f64(pos.into() / TERRAIN_CHUNK_WIDTH_METERS))
}

pub fn spawn_new_chunk(
    world: &mut World,
    index: impl Into<ChunkIndex>,
    color: Color,
) -> Option<Ent> {
    let index = index.into();
    if world.chunk_map.contains_key(&index) {
        return None;
    }

    let chunk = TerrainChunk::new(index, color);
    let id = world.spawner.spawn();
    world.chunks.spawn(id, chunk);
    world.chunk_map.insert(index, id);

    Some(id)
}

pub fn ensure_chunk_exists(world: &mut World, index: ChunkIndex) {
    spawn_new_chunk(world, index, Color::GREEN);
}

pub fn remove_chunk_if_empty(world: &mut World, id: Ent, index: ChunkIndex) -> Option<()> {
    let chunk = world.chunks.get(id)?;
    if !chunk.is_empty() {
        return Some(());
    }

    _ = world.chunks.despawn(id);
    world.chunk_map.remove(&index);

    Some(())
}

pub fn chunk_register_track(world: &mut World, index: ChunkIndex, track_id: Ent) -> Option<()> {
    ensure_chunk_exists(world, index);
    let chunk_id = world.chunk_map.get(&index)?;
    let chunk = world.chunks.try_get_mut(*chunk_id).ok()?;
    chunk.add_track(track_id);
    Some(())
}

pub fn chunk_deregister_track(world: &mut World, index: ChunkIndex, track_id: Ent) -> Option<()> {
    let chunk_id = world.chunk_map.get(&index)?;
    let chunk = world.chunks.try_get_mut(*chunk_id).ok()?;
    chunk.remove_track(track_id);
    remove_chunk_if_empty(world, *chunk_id, index);
    Some(())
}

pub fn chunk_register_node(world: &mut World, index: ChunkIndex, node_id: Ent) -> Option<()> {
    ensure_chunk_exists(world, index);
    let chunk_id = world.chunk_map.get(&index)?;
    let chunk = world.chunks.try_get_mut(*chunk_id).ok()?;
    chunk.add_node(node_id);
    Some(())
}

pub fn chunk_deregister_node(world: &mut World, index: ChunkIndex, node_id: Ent) -> Option<()> {
    let chunk_id = world.chunk_map.get(&index)?;
    let chunk = world.chunks.try_get_mut(*chunk_id).ok()?;
    chunk.remove_node(node_id);
    remove_chunk_if_empty(world, *chunk_id, index);
    Some(())
}
