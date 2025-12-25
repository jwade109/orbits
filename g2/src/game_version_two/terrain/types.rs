use bevy::prelude::*;
use noise::{NoiseFn, Perlin, Seedable, Simplex};
use std::collections::HashMap;

use super::constants::*;
use super::utils::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct OreDeposit;

#[derive(Component, Debug, Clone, Copy)]
pub struct Excavator {
    pub is_enabled: bool,
    pub radius: f32,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct ChunkMap(HashMap<IVec2, Entity>);

impl ChunkMap {
    pub fn lup(&self, pos: Vec2) -> Option<Entity> {
        let g = to_grid(pos);
        self.0.get(&g).cloned()
    }
}

#[derive(Component, Debug)]
pub struct TerrainChunk {
    pub pos: IVec2,
    pub dense: Option<DenseChunkData>,
    pub needs_mesh_update: bool,
}

impl TerrainChunk {
    pub fn is_empty(&self) -> bool {
        self.dense.as_ref().map(|d| d.is_empty()).unwrap_or(true)
    }

    pub fn is_occupied(&self, l: IVec2) -> bool {
        if l.x < 0
            || l.x >= LATTICE_POINTS_PER_CHUNK_SIDE as i32
            || l.y < 0
            || l.y >= LATTICE_POINTS_PER_CHUNK_SIDE as i32
        {
            return true;
        }
        let dense = match &self.dense {
            Some(d) => d,
            _ => return true,
        };
        let x = l.x as usize;
        let y = l.y as usize;

        dense.points[x][y] > 0.5
    }
}

#[derive(Debug)]
pub struct DenseChunkData {
    pub points: [[f32; LATTICE_POINTS_PER_CHUNK_SIDE]; LATTICE_POINTS_PER_CHUNK_SIDE],
}

fn asteroid_field(simplex: &Simplex, pos: Vec2) -> f32 {
    let noise = simplex.get([pos.x as f64 / 100.0, pos.y as f64 / 100.0, 0.0]);
    (noise as f32 + 0.5) * 0.3 + 0.7
}

impl DenseChunkData {
    pub fn new(pos: IVec2, z: f32) -> Self {
        let simplex = Simplex::new(1);

        let mut ret = Self {
            points: [[0.0; LATTICE_POINTS_PER_CHUNK_SIDE]; LATTICE_POINTS_PER_CHUNK_SIDE],
        };

        for x in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
            for y in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
                let p_world = lattice_point_world_pos(pos, IVec2::new(x as i32, y as i32));
                ret.points[x][y] = asteroid_field(&simplex, p_world);
            }
        }

        ret
    }

    pub fn solid() -> Self {
        Self {
            points: [[1.0; LATTICE_POINTS_PER_CHUNK_SIDE]; LATTICE_POINTS_PER_CHUNK_SIDE],
        }
    }

    pub fn is_solid(&self) -> bool {
        self.points.iter().all(|arr| arr.iter().all(|x| *x > 0.8))
    }

    pub fn is_empty(&self) -> bool {
        self.points.iter().all(|arr| arr.iter().all(|x| *x < 0.5))
    }
}
