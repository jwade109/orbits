use crate::aabb::*;
use crate::math::*;
use splines::Spline;

pub const TILES_PER_CHUNK_SIDE: usize = 20;

pub const CHUNK_WIDTH_METERS: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Air,
    Grass,
    Stone,
    Sand,
    Ore,
    DeepStone,
}

impl Tile {
    pub fn is_deep_stone(&self) -> bool {
        match self {
            Self::DeepStone => true,
            _ => false,
        }
    }

    pub fn is_air(&self) -> bool {
        match self {
            Self::Air => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TerrainChunk {
    values: [Tile; TILES_PER_CHUNK_SIDE * TILES_PER_CHUNK_SIDE],
}

impl TerrainChunk {
    pub fn new() -> Self {
        Self {
            values: [Tile::Air; TILES_PER_CHUNK_SIDE * TILES_PER_CHUNK_SIDE],
        }
    }

    pub fn with_elevation(elevation: &Spline<f32, f32>, chunk_pos: IVec2) -> Self {
        let mut chunk = TerrainChunk::new();
        for (i, val) in chunk.values.iter_mut().enumerate() {
            let tile_pos = tile_index_to_pos(i);
            let aabb = tile_pos_to_bounds(chunk_pos, tile_pos);
            if let Some(e) = elevation.clamped_sample(aabb.center.x) {
                let depth = aabb.center.y - e;
                *val = if depth > 0.0 {
                    Tile::Air
                } else if depth > -1.0 {
                    Tile::Grass
                } else if depth > -5.0 {
                    if rand(0.0, 1.0) < 0.7 {
                        Tile::Stone
                    } else {
                        Tile::Sand
                    }
                } else {
                    Tile::DeepStone
                };
            }
        }
        chunk
    }

    pub fn tiles(&self) -> impl Iterator<Item = (UVec2, Tile)> + use<'_> {
        self.values
            .iter()
            .enumerate()
            .map(|(i, e)| (tile_index_to_pos(i), *e))
    }

    pub fn is_deep(&self) -> bool {
        self.values.iter().all(|e| e.is_deep_stone())
    }

    pub fn is_air(&self) -> bool {
        self.values.iter().all(|e| e.is_air())
    }
}

pub fn world_pos_to_chunk(pos: Vec2) -> IVec2 {
    vfloor(pos / CHUNK_WIDTH_METERS)
}

pub fn chunk_pos_to_bounds(pos: IVec2) -> AABB {
    let lower = pos.as_vec2() * CHUNK_WIDTH_METERS;
    let upper = (pos + IVec2::ONE).as_vec2() * CHUNK_WIDTH_METERS;
    AABB::from_arbitrary(lower, upper)
}

pub fn tile_index_to_pos(idx: usize) -> UVec2 {
    let x = idx % TILES_PER_CHUNK_SIDE;
    let y = idx / TILES_PER_CHUNK_SIDE;
    UVec2::new(x as u32, y as u32)
}

pub fn tile_pos_to_bounds(chunk_pos: IVec2, tile_pos: UVec2) -> AABB {
    let lower_chunk = chunk_pos.as_vec2() * CHUNK_WIDTH_METERS;
    let scale = Vec2::splat(CHUNK_WIDTH_METERS) / TILES_PER_CHUNK_SIDE as f32;
    let tile_coords = tile_pos.as_vec2() * scale;
    AABB::from_arbitrary(lower_chunk + tile_coords, lower_chunk + tile_coords + scale)
}
