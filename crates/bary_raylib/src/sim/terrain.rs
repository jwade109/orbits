use std::collections::BTreeMap;

use bary_core::prelude::*;
use bary_orbital::Asteroid;
use early_returns::ok_or_continue;

use crate::sim::*;

pub const TERRAIN_TILE_WIDTH_METERS: f32 = 0.5;
pub const PIXELS_IN_TERRAIN_TILE: u8 = 20;
pub const TILES_PER_CHUNK_SIDE: u8 = 32;
pub const TERRAIN_CHUNK_WIDTH_METERS: f32 = TERRAIN_TILE_WIDTH_METERS * TILES_PER_CHUNK_SIDE as f32;
pub const TERRAIN_VARIANTS: u8 = 8;

#[derive(Debug, Clone, Copy)]
pub enum TerrainMaterial {
    Rock,
    Dirt,
    Ice,
    Silicon,
    Iron,
}

impl TerrainMaterial {
    pub fn random() -> Self {
        let n = randint(0, 9);
        match n {
            0..5 => TerrainMaterial::Rock,
            5 => TerrainMaterial::Dirt,
            6 => TerrainMaterial::Ice,
            7 => TerrainMaterial::Silicon,
            8 => TerrainMaterial::Iron,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTileIndex(pub I8Vec2);

impl LocalTileIndex {
    pub fn origin_isometry(&self) -> Isometry2d {
        let bottom_left = self.0.as_vec2() * TERRAIN_TILE_WIDTH_METERS as f32;
        Isometry2d::from_pos(bottom_left)
    }

    pub fn center_isometry(&self) -> Isometry2d {
        let bottom_left = (self.0.as_vec2() + Vec2::splat(0.5)) * TERRAIN_TILE_WIDTH_METERS as f32;
        Isometry2d::from_pos(bottom_left)
    }

    pub fn top_left_isometry(&self) -> Isometry2d {
        let idx = self.0 + I8Vec2::Y;
        let top_left = idx.as_vec2() * TERRAIN_TILE_WIDTH_METERS as f32;
        Isometry2d::from_pos(top_left)
    }

    pub fn center(&self) -> Vec2 {
        let bl = self.origin_isometry().translation;
        bl + Vec2::splat(TERRAIN_TILE_WIDTH_METERS as f32 / 2.0)
    }

    pub fn bb(&self) -> AABB {
        AABB::new(self.center(), Vec2::splat(TERRAIN_TILE_WIDTH_METERS as f32))
    }
}

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

    pub fn center_isometry(&self) -> Isometry2d {
        Isometry2d::from_pos(self.center())
    }

    pub fn bb(&self) -> AABB {
        AABB::new(
            self.center(),
            Vec2::splat(TERRAIN_CHUNK_WIDTH_METERS as f32),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainIndex(pub IVec2, pub I8Vec2);

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
    pub visible_count: usize,
}

impl TerrainChunk {
    fn new(parent: Ent, index: ChunkIndex) -> Self {
        Self {
            parent,
            index,
            tiles: BTreeMap::new(),
            visible_count: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TerrainTile {
    parent: Ent,
    material: TerrainMaterial,
    variant: usize,
    light_level: u8,
}

impl TerrainTile {
    fn new(parent: Ent, material: TerrainMaterial, is_edge: bool) -> Self {
        Self {
            parent,
            material,
            variant: randint(0, TERRAIN_VARIANTS as i32) as usize,
            light_level: 0,
        }
    }

    pub fn parent(&self) -> Ent {
        self.parent
    }

    pub fn material(&self) -> TerrainMaterial {
        self.material
    }

    pub fn variant(&self) -> usize {
        self.variant
    }

    pub fn light_level(&self) -> u8 {
        self.light_level
    }

    pub fn is_visible(&self) -> bool {
        self.light_level > 0
    }
}

pub fn spawn_asteroid(world: &mut World, ast: Asteroid, iso: Isometry2d) -> Ent {
    let parent = world.spawner.spawn();

    let rmax = (ast.max_radius() / TERRAIN_CHUNK_WIDTH_METERS as f32).ceil() as i32;

    let mut chunks = BTreeMap::new();
    let mut edges = Vec::new();

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
                    let index = LocalTileIndex(I8Vec2::new(j as i8, k as i8));
                    let iso = chunk_index.origin_isometry() * index.center_isometry();
                    if !ast.contains(iso.translation) {
                        continue;
                    }

                    let is_edge = !index.bb().corners().iter().all(|c| {
                        let c = chunk_index.origin_isometry() * Isometry2d::from_pos(*c);
                        ast.contains(c.translation)
                    });

                    let tile = TerrainTile::new(parent, TerrainMaterial::random(), is_edge);

                    let tile_id = world.spawner.spawn();
                    world.terrain_tiles.spawn(tile_id, tile);
                    tiles.insert(index, tile_id);

                    if tile.is_visible() {
                        chunk.visible_count += 1;
                    }

                    if is_edge {
                        edges.push((chunk_index, index));
                    }
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

pub fn remove_terrain_tile(
    world: &mut World,
    ast_id: Ent,
    cidx: ChunkIndex,
    tidx: LocalTileIndex,
) -> BaryResult<Ent> {
    let rock = world.asteroids.try_get_mut(ast_id)?;
    let chunk_id = *rock.chunks.get(&cidx).ok_or(BaryError::NoChunk)?;
    let chunk = world.terrain_chunks.try_get_mut(chunk_id)?;
    let tile_id = *chunk.tiles.get(&tidx).ok_or(BaryError::NoTile)?;

    world.terrain_tiles.despawn(tile_id)?;

    chunk.tiles.remove(&tidx);
    if chunk.tiles.is_empty() {
        world.terrain_chunks.despawn(chunk_id)?;
        rock.chunks.remove(&cidx);
        if rock.chunks.is_empty() {
            world.asteroids.despawn(ast_id)?;
        }
    }

    light_up_terrain_tile(world, ast_id, cidx, tidx)?;

    Ok(tile_id)
}

pub fn add_terrain_tile(
    world: &mut World,
    ast_id: Ent,
    cidx: ChunkIndex,
    tidx: LocalTileIndex,
) -> BaryResult<Ent> {
    let rock = world.asteroids.try_get_mut(ast_id)?;
    let chunk_id = *rock.chunks.get(&cidx).ok_or(BaryError::NoChunk)?;
    let chunk = world.terrain_chunks.try_get_mut(chunk_id)?;

    if chunk.tiles.contains_key(&tidx) {
        return Err(BaryError::TileAlreadyExists);
    }

    let tile_id = world.spawner.spawn();
    let tile = TerrainTile::new(chunk_id, TerrainMaterial::random(), true);
    chunk.tiles.insert(tidx, tile_id);
    if tile.is_visible() {
        chunk.visible_count += 1;
    }

    world.terrain_tiles.spawn(tile_id, tile);

    Ok(tile_id)
}

pub fn light_up_terrain_tile(
    world: &mut World,
    ast_id: Ent,
    cidx: ChunkIndex,
    tidx: LocalTileIndex,
) -> BaryResult<()> {
    let rock = world.asteroids.try_get_mut(ast_id)?;
    let chunk_id = *rock.chunks.get(&cidx).ok_or(BaryError::NoChunk)?;
    let chunk = world.terrain_chunks.try_get_mut(chunk_id)?;

    let rmax = 8;

    for x in -rmax..=rmax {
        for y in -rmax..=rmax {
            let d = I8Vec2::new(x, y);
            let r = d.as_vec2().length().round() as i8;
            let ll = if r < 8 { 8 - r as u8 } else { 0 };
            let c = LocalTileIndex(tidx.0 + d);
            let tile_id = ok_or_continue!(chunk.tiles.get(&c).ok_or(BaryError::NoTile));
            let tile = world.terrain_tiles.try_get_mut(*tile_id)?;
            let visible = tile.is_visible();
            tile.light_level = tile.light_level.max(ll);
            if !visible && tile.is_visible() {
                chunk.visible_count += 1;
            }
        }
    }

    Ok(())
}
