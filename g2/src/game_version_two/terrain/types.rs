use bevy::color::palettes::css::*;
use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use game::starling::prelude::*;
use noise::{NoiseFn, Perlin, Seedable, Simplex};
use std::collections::{HashMap, HashSet};

use super::constants::*;
use super::utils::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
            Substrate::Rock => Item::Stone,
            Substrate::Dirt => Item::Ice,
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

#[derive(Component, Debug, Clone)]
pub struct Excavator {
    pub is_on: bool,
    pub forward_offset: f32,
    pub radius: f32,
    pub timer: Timer,
    pub status: MachineStatus,
    pub last_op_status: MachineStatus,
}

impl Excavator {
    pub fn new(radius: f32) -> Self {
        Self {
            is_on: false,
            radius,
            forward_offset: 2.0,
            timer: Timer::from_seconds(0.2, TimerMode::Repeating),
            status: MachineStatus::Off,
            last_op_status: MachineStatus::Off,
        }
    }

    pub fn effector_center(&self, transform: &GlobalTransform) -> Vec3 {
        transform.translation() + transform.right() * self.forward_offset
    }
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

    pub fn tile(&self, l: IVec2) -> Option<&Tile> {
        self.dense.as_ref().map(|d| d.tile(l)).flatten()
    }

    pub fn mass(&self) -> Mass {
        self.dense
            .as_ref()
            .map(|d| d.iter_tiles().map(|t| t.mass).sum())
            .unwrap_or(Mass::ZERO)
    }

    pub fn substrates(&self) -> HashSet<Substrate> {
        let mut set = HashSet::new();
        for tile in self.iter_tiles() {
            set.insert(tile.substrate);
        }
        set
    }

    pub fn mass_of(&self, substrate: Substrate) -> Mass {
        self.dense
            .as_ref()
            .map(|d| {
                d.iter_tiles()
                    .map(|t| {
                        if t.substrate == substrate {
                            t.mass
                        } else {
                            Mass::ZERO
                        }
                    })
                    .sum()
            })
            .unwrap_or(Mass::ZERO)
    }

    pub fn iter_tiles(&self) -> impl Iterator<Item = &Tile> + use<'_> {
        self.dense.iter().flat_map(|d| d.iter_tiles())
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

    pub fn has_some(&self, substrate: Substrate) -> bool {
        self.substrate == substrate && !self.mass.is_zero()
    }

    /// reduces mass of this tile by delta.
    /// return true if the tile was reduced to zero mass.
    pub fn mine(&mut self, delta: Mass) -> bool {
        if self.mass.is_zero() {
            false
        } else if self.mass <= delta {
            self.mass = Mass::ZERO;
            true
        } else {
            self.mass -= delta;
            false
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

fn asteroid_field(simplex: &Simplex, pos: Vec2, asteroid: &Asteroid) -> Mass {
    let r = if pos.length() > 0.0 {
        asteroid.radius_at(pos.to_angle())
    } else {
        0.0
    };

    let noise = simplex.get([pos.x as f64 / 100.0, pos.y as f64 / 100.0, 0.0]);
    let kg = (noise as f32 + 0.5) * 0.7 + 0.3;
    let kg = if pos.length() < r { kg } else { 0.0 };
    Mass::from_kg_f32(kg * 1200.0)
}

#[derive(Debug)]
pub struct DenseChunkData {
    pub points: [[Tile; TILES_PER_CHUNK_SIDE]; TILES_PER_CHUNK_SIDE],
}

impl DenseChunkData {
    pub fn new(pos: IVec2, z: f32, asteroid: &Asteroid) -> Self {
        let simplex = Simplex::new(1);

        let mut ret = Self {
            points: [[Tile::default(); TILES_PER_CHUNK_SIDE]; TILES_PER_CHUNK_SIDE],
        };

        for x in 0..TILES_PER_CHUNK_SIDE {
            for y in 0..TILES_PER_CHUNK_SIDE {
                let p_world = lattice_point_center_world_pos(pos, IVec2::new(x as i32, y as i32));
                let mass = asteroid_field(&simplex, p_world, asteroid);

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

    pub fn tile(&self, l: IVec2) -> Option<&Tile> {
        if l.x < 0
            || l.x >= TILES_PER_CHUNK_SIDE as i32
            || l.y < 0
            || l.y >= TILES_PER_CHUNK_SIDE as i32
        {
            return None;
        }
        let x = l.x as usize;
        let y = l.y as usize;

        Some(&self.points[x][y])
    }

    /// iterate over all tiles in indeterminate order.
    /// it's not like the order is unknowable, I just don't care to specify it.
    pub fn iter_tiles(&self) -> impl Iterator<Item = &Tile> + use<'_> {
        self.points.iter().flat_map(|row| row.iter())
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

#[derive(Component)]
pub struct MiningIndicator {
    pub remaining: Timer,
    pub success: MiningFailure,
    pub pos: Vec2,
}

#[derive(Resource, Deref, DerefMut)]
pub struct Ast(pub Asteroid);

#[derive(Debug, Clone, Copy)]
pub enum MiningFailure {
    Ok,
    NoRoom,
    NoMaterial,
}
