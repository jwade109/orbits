use std::collections::BTreeMap;

use bary_core::prelude::*;
use bary_orbital::Asteroid;

use crate::sim::*;

pub const TERRAIN_TILE_WIDTH_METERS: f32 = 0.5;
pub const PIXELS_IN_TERRAIN_TILE: f32 = 20.0;
pub const TILES_PER_CHUNK_SIDE: u8 = 32;
pub const TERRAIN_CHUNK_WIDTH_METERS: f32 = TERRAIN_TILE_WIDTH_METERS * TILES_PER_CHUNK_SIDE as f32;

#[derive(Debug, Clone, Copy)]
pub enum TerrainMaterial {
    Dirt,
    Rock,
    Ice,
    Silicon,
    Iron,
}

impl TerrainMaterial {
    pub fn random() -> Self {
        let n = randint(0, 5);
        match n {
            0 => TerrainMaterial::Dirt,
            1 => TerrainMaterial::Rock,
            2 => TerrainMaterial::Ice,
            3 => TerrainMaterial::Silicon,
            4 => TerrainMaterial::Iron,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTileIndex(pub U8Vec2);

impl PartialOrd for LocalTileIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.0.x, self.0.y).partial_cmp(&(other.0.x, other.0.y))
    }
}

impl Ord for LocalTileIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0.x, self.0.y).cmp(&(other.0.x, other.0.y))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalTileIndex(pub IVec2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkIndex(pub IVec2);

impl PartialOrd for ChunkIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.0.x, self.0.y).partial_cmp(&(other.0.x, other.0.y))
    }
}

impl Ord for ChunkIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0.x, self.0.y).cmp(&(other.0.x, other.0.y))
    }
}

impl ChunkIndex {
    pub fn origin_isometry(&self) -> Isometry2d {
        let bottom_left = self.0.as_vec2() * TERRAIN_CHUNK_WIDTH_METERS as f32;
        Isometry2d::from_pos(bottom_left)
    }

    pub fn top_left_isometry(&self) -> Isometry2d {
        let idx = self.0 + IVec2::Y;
        let top_left = idx.as_vec2() * TERRAIN_CHUNK_WIDTH_METERS as f32;
        Isometry2d::from_pos(top_left)
    }

    pub fn center(&self) -> Vec2 {
        let bl = self.origin_isometry().translation;
        bl + Vec2::splat(TERRAIN_CHUNK_WIDTH_METERS as f32 / 2.0)
    }

    pub fn bb(&self) -> AABB {
        AABB::new(
            self.center(),
            Vec2::splat(TERRAIN_CHUNK_WIDTH_METERS as f32),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainIndex(pub IVec2, pub U8Vec2);

#[derive(Debug, Clone)]
pub struct BigRock {
    pub iso: Isometry2d,
    pub ast: Asteroid,
    pub chunks: BTreeMap<ChunkIndex, Ent>,
}

#[derive(Debug, Clone)]
pub struct TerrainChunk {
    pub parent: Ent,
    pub index: ChunkIndex,
    pub tiles: BTreeMap<LocalTileIndex, Ent>,
}

impl TerrainChunk {
    fn new(parent: Ent, index: ChunkIndex) -> Self {
        Self {
            parent,
            index,
            tiles: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TerrainTile {
    parent: Ent,
    index: LocalTileIndex,
    material: TerrainMaterial,
}

impl TerrainTile {
    fn new(parent: Ent, index: impl Into<LocalTileIndex>) -> Self {
        Self {
            parent,
            index: index.into(),
            material: TerrainMaterial::random(),
        }
    }

    pub fn parent(&self) -> Ent {
        self.parent
    }

    pub fn material(&self) -> TerrainMaterial {
        self.material
    }

    pub fn origin_isometry(&self) -> Isometry2d {
        let bottom_left = self.index.0.as_vec2() * TERRAIN_TILE_WIDTH_METERS as f32;
        Isometry2d::from_pos(bottom_left)
    }

    pub fn center_isometry(&self) -> Isometry2d {
        let bottom_left = (self.index.0.as_vec2() + Vec2::splat(0.5)) * TERRAIN_TILE_WIDTH_METERS as f32;
        Isometry2d::from_pos(bottom_left)
    }

    pub fn top_left_isometry(&self) -> Isometry2d {
        let idx = self.index.0 + U8Vec2::Y;
        let top_left = idx.as_vec2() * TERRAIN_TILE_WIDTH_METERS as f32;
        Isometry2d::from_pos(top_left)
    }

    pub fn center(&self) -> Vec2 {
        let bl = self.origin_isometry().translation;
        bl + Vec2::splat(TERRAIN_TILE_WIDTH_METERS as f32 / 2.0)
    }
}

pub fn spawn_asteroid(world: &mut World, ast: Asteroid, iso: Isometry2d) -> Ent {
    let parent = world.spawner.spawn();

    let rmax = (ast.max_radius() / TERRAIN_CHUNK_WIDTH_METERS as f32).ceil() as i32;

    let mut chunks = BTreeMap::new();

    for x in -rmax..rmax {
        for y in -rmax..rmax {
            let chunk_index = ChunkIndex(IVec2::new(x, y));
            let mut chunk = TerrainChunk::new(parent, chunk_index);

            let is_intersecting = chunk_index
                .bb()
                .corners()
                .iter()
                .any(|c| c.length() < ast.max_radius());

            if !is_intersecting {
                continue;
            }

            let mut tiles = BTreeMap::new();
            for j in 0..TILES_PER_CHUNK_SIDE {
                for k in 0..TILES_PER_CHUNK_SIDE {
                    let index = LocalTileIndex(U8Vec2::new(j, k));
                    let tile = TerrainTile::new(parent, index);
                    let iso = chunk_index.origin_isometry() * tile.center_isometry();
                    if !ast.contains(iso.translation) {
                        continue;
                    }
                    let tile_id = world.spawner.spawn();
                    world.terrain_tiles.spawn(tile_id, tile);
                    tiles.insert(index, tile_id);
                }
            }

            if tiles.is_empty() {
                continue;
            }

            chunk.tiles = tiles;

            let chunk_id = world.spawner.spawn();
            world.terrain_chunks.spawn(chunk_id, chunk);
            chunks.insert(chunk_index, chunk_id);
        }
    }

    let rock = BigRock { ast, iso, chunks };

    world.asteroids.spawn(parent, rock);

    parent
}

pub fn spawn_random_asteroid(world: &mut World, iso: Isometry2d, radius: f32, seed: u64) {
    let ast = Asteroid::random(radius, Some(seed));
    spawn_asteroid(world, ast, iso);
}

pub fn get_lod_bounding_boxes(ast: &Asteroid, lod: u32) -> Vec<(IVec2, AABB)> {
    let length = lod_to_length(lod) as f32;

    if length < 1.0 {
        return vec![];
    }

    let max_i = (ast.max_radius() / length).ceil() as i32;

    if max_i < 1 {
        return vec![];
    }

    let size = Vec2::splat(length);

    let mut ret = Vec::new();

    for x in -max_i..max_i {
        for y in -max_i..max_i {
            let idx = IVec2::new(x, y);
            let lower = idx.as_vec2() * length;
            let upper = lower + size;
            let bb = AABB::from_arbitrary(lower, upper);

            let p = bb.corners();

            if p.iter().any(|p| ast.contains(*p)) {
                ret.push((idx, bb));
            }
        }
    }

    ret
}

pub fn lod_to_length(lod: u32) -> i32 {
    2i32.pow(lod as u32)
}

pub fn length_to_lod(length: f32) -> u32 {
    length.log2().round() as u32
}
