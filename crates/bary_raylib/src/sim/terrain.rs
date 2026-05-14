use bary_core::prelude::*;
use bary_orbital::Asteroid;

use crate::sim::*;

pub const TERRAIN_TILE_WIDTH: u32 = 1;
pub const PIXELS_IN_TERRAIN_TILE: f32 = 20.0;
pub const TILES_PER_CHUNK_SIDE: u32 = 32;
pub const TERRAIN_CHUNK_WIDTH_METERS: u32 = TERRAIN_TILE_WIDTH * TILES_PER_CHUNK_SIDE;

#[derive(Debug, Clone, Copy)]
pub enum TerrainMaterial {
    Dirt,
    Rock,
    Ice,
    Silicon,
    Iron,
    Nickel,
}

impl TerrainMaterial {
    pub fn random() -> Self {
        let n = randint(0, 6);
        match n {
            0 => TerrainMaterial::Dirt,
            1 => TerrainMaterial::Rock,
            2 => TerrainMaterial::Ice,
            3 => TerrainMaterial::Silicon,
            4 => TerrainMaterial::Iron,
            5 => TerrainMaterial::Nickel,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BigRock {
    pub iso: Isometry2d,
    pub ast: Asteroid,
    pub chunks: Vec<Ent>,
}

#[derive(Debug, Clone)]
pub struct TerrainChunk {
    parent: Ent,
    index: IVec2,
    tiles: Vec<Ent>,
}

impl TerrainChunk {
    fn new(parent: Ent, index: impl Into<IVec2>, tiles: Vec<Ent>) -> Self {
        Self {
            parent,
            index: index.into(),
            tiles,
        }
    }

    pub fn origin_isometry(&self) -> Isometry2d {
        let bottom_left = self.index.as_vec2() * TERRAIN_CHUNK_WIDTH_METERS as f32;
        Isometry2d::from_pos(bottom_left)
    }

    pub fn top_left_isometry(&self) -> Isometry2d {
        let idx = self.index + IVec2::Y;
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

#[derive(Debug, Clone, Copy)]
pub struct TerrainTile {
    parent: Ent,
    index: UVec2,
    material: TerrainMaterial,
}

impl TerrainTile {
    fn new(parent: Ent, index: impl Into<UVec2>) -> Self {
        Self {
            parent,
            index: index.into(),
            material: TerrainMaterial::random(),
        }
    }

    pub fn parent(&self) -> Ent {
        self.parent
    }

    pub fn index(&self) -> UVec2 {
        self.index
    }

    pub fn material(&self) -> TerrainMaterial {
        self.material
    }

    pub fn origin_isometry(&self) -> Isometry2d {
        let bottom_left = self.index.as_vec2() * TERRAIN_TILE_WIDTH as f32;
        Isometry2d::from_pos(bottom_left)
    }

    pub fn top_left_isometry(&self) -> Isometry2d {
        let idx = self.index + UVec2::Y;
        let top_left = idx.as_vec2() * TERRAIN_TILE_WIDTH as f32;
        Isometry2d::from_pos(top_left)
    }

    pub fn center(&self) -> Vec2 {
        let bl = self.origin_isometry().translation;
        bl + Vec2::splat(TERRAIN_TILE_WIDTH as f32 / 2.0)
    }
}

pub fn spawn_asteroid(world: &mut World, ast: Asteroid, iso: Isometry2d) -> Ent {
    let parent = world.spawner.spawn();

    let rmax = (ast.max_radius() / TERRAIN_CHUNK_WIDTH_METERS as f32).ceil() as i32;

    let mut chunks = Vec::new();

    for x in -rmax..rmax {
        for y in -rmax..rmax {
            let mut chunk = TerrainChunk::new(parent, (x, y), vec![]);

            let is_intersecting = chunk
                .bb()
                .corners()
                .iter()
                .any(|c| c.length() < ast.max_radius());

            if !is_intersecting {
                continue;
            }

            let mut tiles = Vec::new();
            for j in 0..TILES_PER_CHUNK_SIDE {
                for k in 0..TILES_PER_CHUNK_SIDE {
                    let tile = TerrainTile::new(parent, (j, k));
                    let tile_id = world.spawner.spawn();
                    world.terrain_tiles.spawn(tile_id, tile);
                    tiles.push(tile_id);
                }
            }

            chunk.tiles = tiles;

            let chunk_id = world.spawner.spawn();
            world.terrain_chunks.spawn(chunk_id, chunk);
            chunks.push(chunk_id);
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
