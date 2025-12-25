use bevy::color::palettes::css::*;
use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use game::starling::factory::Item;
use game::starling::math::chance;
use game::starling::units::Mass;
use noise::{NoiseFn, Perlin, Seedable, Simplex};
use std::collections::HashMap;

use super::constants::*;
use super::utils::*;

#[derive(Debug, Clone, Copy)]
pub enum Substrate {
    Rock,
    Dirt,
    IronOre,
    CopperOre,
    UraniumOre,
}

impl Substrate {
    pub fn yields(&self) -> Item {
        match self {
            Substrate::Rock => Item::Geodes,
            Substrate::Dirt => Item::Bread,
            Substrate::IronOre => Item::Iron,
            Substrate::CopperOre => Item::Copper,
            Substrate::UraniumOre => Item::U238,
        }
    }

    pub fn color(&self) -> Srgba {
        match self {
            Substrate::Rock => DARK_SLATE_GRAY,
            Substrate::Dirt => BROWN,
            Substrate::IronOre => SILVER,
            Substrate::CopperOre => GREEN_700,
            Substrate::UraniumOre => GREEN_400,
        }
    }
}

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
            || l.x >= TILES_PER_CHUNK_SIDE as i32
            || l.y < 0
            || l.y >= TILES_PER_CHUNK_SIDE as i32
        {
            return true;
        }

        let dense = match &self.dense {
            Some(d) => d,
            _ => return true,
        };
        let x = l.x as usize;
        let y = l.y as usize;

        dense.points[x][y].mass > Mass::ZERO
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub substrate: Substrate,
    pub mass: Mass,
}

impl Tile {
    pub fn apply_delta_mass(&mut self, delta_mass_kg: f32) {
        if delta_mass_kg > 0.0 {
            let delta = Mass::from_kg_f32(delta_mass_kg);
        }
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            substrate: Substrate::Rock,
            mass: Mass::ZERO,
        }
    }
}

#[derive(Debug)]
pub struct DenseChunkData {
    pub points: [[Tile; TILES_PER_CHUNK_SIDE]; TILES_PER_CHUNK_SIDE],
}

fn asteroid_field(simplex: &Simplex, pos: Vec2) -> Mass {
    let kg = if pos.length() < 300.0 { 10000.0 } else { 0.0 };
    // let noise = simplex.get([pos.x as f64 / 100.0, pos.y as f64 / 100.0, 0.0]);
    // let kg = (noise as f32 + 0.5) * 0.3 + 0.7;
    Mass::from_kg_f32(kg)
}

impl DenseChunkData {
    pub fn new(pos: IVec2, z: f32) -> Self {
        let simplex = Simplex::new(1);

        let mut ret = Self {
            points: [[Tile::default(); TILES_PER_CHUNK_SIDE]; TILES_PER_CHUNK_SIDE],
        };

        for x in 0..TILES_PER_CHUNK_SIDE {
            for y in 0..TILES_PER_CHUNK_SIDE {
                let p_world = lattice_point_center_world_pos(pos, IVec2::new(x as i32, y as i32));
                let mass = asteroid_field(&simplex, p_world);

                let substrate = if chance(0.8) {
                    Substrate::Rock
                } else if chance(0.5) {
                    Substrate::Dirt
                } else {
                    Substrate::IronOre
                };

                let tile = Tile { substrate, mass };
                ret.points[x][y] = tile;
            }
        }

        ret
    }

    pub fn is_solid(&self) -> bool {
        self.points
            .iter()
            .all(|arr| arr.iter().all(|x| x.mass > Mass::ZERO))
    }

    pub fn is_empty(&self) -> bool {
        self.points
            .iter()
            .all(|arr| arr.iter().all(|x| x.mass == Mass::ZERO))
    }
}
