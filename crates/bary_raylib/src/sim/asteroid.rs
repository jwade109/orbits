use bary_core::prelude::*;
use bary_orbital::Asteroid;
use rand::random;

use crate::sim::*;

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
    pub tiles: Vec<Ent>,
}

#[derive(Debug, Clone, Copy)]
pub struct TerrainTile {
    parent: Ent,
    index: IVec2,
    material: TerrainMaterial,
}

impl TerrainTile {
    fn new(parent: Ent, index: impl Into<IVec2>) -> Self {
        Self {
            parent,
            index: index.into(),
            material: TerrainMaterial::random(),
        }
    }

    pub fn parent(&self) -> Ent {
        self.parent
    }

    pub fn index(&self) -> IVec2 {
        self.index
    }

    pub fn material(&self) -> TerrainMaterial {
        self.material
    }

    pub fn origin_isometry(&self) -> Isometry2d {
        let bottom_left = self.index.as_vec2() * TERRAIN_TILE_WIDTH;
        Isometry2d::from_pos(bottom_left)
    }

    pub fn top_left_isometry(&self) -> Isometry2d {
        let idx = self.index + IVec2::Y;
        let top_left = idx.as_vec2() * TERRAIN_TILE_WIDTH;
        Isometry2d::from_pos(top_left)
    }

    pub fn center(&self) -> Vec2 {
        let bl = self.origin_isometry().translation;
        bl + Vec2::splat(TERRAIN_TILE_WIDTH / 2.0)
    }
}

pub const TERRAIN_TILE_WIDTH: f32 = 1.0;
pub const PIXELS_IN_TERRAIN_TILE: f32 = 20.0;

pub fn spawn_asteroid(world: &mut World, ast: Asteroid, iso: Isometry2d) -> Ent {
    let parent = world.spawner.spawn();

    let rmax = (ast.max_radius() / TERRAIN_TILE_WIDTH).ceil() as i32;

    let mut tiles = Vec::new();

    for x in -rmax..=rmax {
        for y in -rmax..=rmax {
            let tile = TerrainTile::new(parent, (x, y));
            let center = tile.center();
            if !ast.contains(center) {
                continue;
            }
            let id = world.spawner.spawn();
            world.terrain_tiles.spawn(id, tile);
            tiles.push(id);
        }
    }

    let rock = BigRock { ast, iso, tiles };

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
