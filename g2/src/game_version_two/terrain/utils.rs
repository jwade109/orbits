use bevy::color::palettes::css::*;
use bevy::prelude::*;
use game::starling::prelude::Inventory;
use game::starling::prelude::Item;
use game::starling::prelude::MachineStatus;
use game::starling::units::Mass;

use crate::game_version_two::MeshMaker;

use super::constants::*;
use super::types::*;

pub fn to_grid(pos: Vec2) -> IVec2 {
    (pos / CHUNK_WIDTH).floor().as_ivec2()
}

pub fn to_grid_and_lattice(pos: Vec2) -> (IVec2, UVec2) {
    let g = to_grid(pos);
    let (lower, upper) = chunk_bounds(g);
    let u = (pos - lower) / CHUNK_WIDTH;

    let lattice_idx = (u * TILES_PER_CHUNK_SIDE as f32).floor().as_uvec2();

    (g, lattice_idx)
}

pub fn chunk_bounds(g: IVec2) -> (Vec2, Vec2) {
    let lower = g.as_vec2() * CHUNK_WIDTH;
    let upper = lower + Vec2::splat(CHUNK_WIDTH);
    (lower, upper)
}

pub fn chunk_center(g: IVec2) -> Vec2 {
    let (lower, upper) = chunk_bounds(g);
    (upper + lower) / 2.0
}

pub fn lattice_point_center_world_pos(g: IVec2, l: IVec2) -> Vec2 {
    let (lower, _) = chunk_bounds(g);
    let off = lattice_point_center_rel_pos(l);
    lower + off
}

pub fn lattice_point_center_rel_pos(l: IVec2) -> Vec2 {
    let xoff = (l.x as f32 + 0.5) / TILES_PER_CHUNK_SIDE as f32 * CHUNK_WIDTH;
    let yoff = (l.y as f32 + 0.5) / TILES_PER_CHUNK_SIDE as f32 * CHUNK_WIDTH;
    Vec2::new(xoff, yoff)
}

pub fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        return 0.0;
    }

    return (value - a) / (b - a);
}

#[deprecated(note = "Temporarily shelving this for simpler rendering.")]
fn marching_cubes_mesh(dense: &DenseChunkData) -> Mesh {
    let mut builder = MeshMaker::default();

    let mass_levels = [Mass::ZERO, Mass::kilograms(500), Mass::kilograms(1000)];

    let square_size = CHUNK_WIDTH / TILES_PER_CHUNK_SIDE as f32;

    for (i, level) in mass_levels.into_iter().enumerate() {
        let color = Srgba::gray(0.4 + i as f32 / 10.0);
        builder.set_color(color);

        for x in 0..TILES_PER_CHUNK_SIDE - 1 {
            for y in 0..TILES_PER_CHUNK_SIDE - 1 {
                let value = dense.points[x][y];

                let xu = x + 1;
                let yu = y + 1;

                let bitmask = ((dense.points[x][y].mass > level) as u8)
                    | ((dense.points[xu][y].mass > level) as u8) << 1
                    | ((dense.points[xu][yu].mass > level) as u8) << 2
                    | ((dense.points[x][yu].mass > level) as u8) << 3;

                let bottom_left = (IVec2::new(x as i32, y as i32).as_vec2()
                    / TILES_PER_CHUNK_SIDE as f32
                    - Vec2::splat(0.5))
                    * CHUNK_WIDTH;

                let bottom_left = bottom_left + Vec2::splat(square_size / 2.0);

                let top_right = bottom_left + Vec2::splat(square_size);
                let bottom_right = bottom_left + Vec2::X * square_size;
                let top_left = bottom_left + Vec2::Y * square_size;

                let left = bottom_left + Vec2::Y * square_size * 0.5;
                let bottom = bottom_left + Vec2::X * square_size * 0.5;
                let right = top_right - Vec2::Y * square_size * 0.5;
                let top = top_right - Vec2::X * square_size * 0.5;

                let to_arr = |p: Vec2| [p.x, p.y, 0.0];

                match bitmask {
                    0 => (),
                    1 => {
                        // bottom left corner
                        builder.triangle([bottom_left, left, bottom]);
                    }
                    2 => {
                        // bottom right corner
                        builder.triangle([bottom_right, right, bottom]);
                    }
                    3 => {
                        // bottom half
                        builder.rectangle([bottom_left, bottom_right, right, left]);
                    }
                    4 => {
                        // top right corner
                        builder.triangle([top_right, right, top]);
                    }
                    5 => {
                        // bottom left corner
                        builder.triangle([bottom_left, left, bottom]);
                        // top right corner
                        builder.triangle([top_right, right, top]);
                    }
                    6 => {
                        // right half
                        builder.rectangle([top_right, bottom_right, bottom, top]);
                    }
                    7 => {
                        builder.pentagon([bottom_left, bottom_right, top_right, top, left]);
                    }
                    8 => {
                        // top left corner
                        builder.triangle([top_left, left, top]);
                    }
                    9 => {
                        // left half
                        builder.rectangle([top_left, bottom_left, bottom, top]);
                    }
                    10 => {
                        // bottom right corner
                        builder.triangle([bottom_right, right, bottom]);
                        // top left corner
                        builder.triangle([top_left, left, top]);
                    }
                    11 => {
                        builder.pentagon([bottom_left, bottom_right, right, top, top_left]);
                    }
                    12 => {
                        builder.rectangle([top_left, top_right, right, left]);
                    }
                    13 => {
                        builder.pentagon([bottom_left, bottom, right, top_right, top_left]);
                    }
                    14 => {
                        builder.pentagon([bottom, bottom_right, top_right, top_left, left]);
                    }
                    _ => {
                        builder.rectangle([bottom_left, bottom_right, top_right, top_left]);
                    }
                }
            }
        }
    }

    builder.build()
}

pub fn simple_mesh(dense: &DenseChunkData) -> Mesh {
    let mut builder = MeshMaker::default();

    for x in 0..TILES_PER_CHUNK_SIDE {
        for y in 0..TILES_PER_CHUNK_SIDE {
            let tile = &dense.points[x][y];

            if tile.mass.is_zero() {
                continue;
            }

            let color = tile.substrate.color();

            builder.set_color(color);

            let l = IVec2::new(x as i32, y as i32);
            let c = lattice_point_center_rel_pos(l);

            let square_size = 1.0 / TILES_PER_CHUNK_SIDE as f32 * CHUNK_WIDTH;

            let bottom_left = (IVec2::new(x as i32, y as i32).as_vec2()
                / TILES_PER_CHUNK_SIDE as f32
                - Vec2::splat(0.5))
                * CHUNK_WIDTH;
            let top_right = bottom_left + Vec2::splat(square_size);
            let bottom_right = bottom_left + Vec2::X * square_size;
            let top_left = bottom_left + Vec2::Y * square_size;

            builder.rectangle([bottom_left, bottom_right, top_right, top_left]);
        }
    }

    builder.build()
}

pub fn generate_mesh_data(chunk: &TerrainChunk) -> (Mesh, Srgba) {
    if let Some(dense) = &chunk.dense {
        if dense.is_empty() {
            (Rectangle::new(0.1, 0.1).into(), RED)
        } else {
            // (simple_mesh(dense), WHITE)
            (marching_cubes_mesh(dense), WHITE)
        }
    } else {
        (Rectangle::from_size(Vec2::splat(CHUNK_WIDTH)).into(), WHITE)
    }
}

pub fn global_to_gl(gl: IVec2) -> (IVec2, IVec2) {
    let n = TILES_PER_CHUNK_SIDE;
    let mut g = IVec2::new(gl.x / n as i32, gl.y / n as i32);
    let mut l = IVec2::new(gl.x % n as i32, gl.y % n as i32);
    if l.x < 0 {
        g.x -= 1;
        l.x += n as i32;
    }
    if l.y < 0 {
        g.y -= 1;
        l.y += n as i32;
    }
    (g, l)
}

/// moves items from a tile to an inventory, if possible,
/// without leaving either in a state where matter is
/// created or destroyed
pub fn atomic_mine(src: &mut Tile, dst: &mut Inventory, item: Item, count: u64) -> MachineStatus {
    if !dst.can_store(item, count) {
        return MachineStatus::NoRoom;
    }

    if src.mass.is_zero() {
        return MachineStatus::Starved;
    }

    let mass = item.mass_per_unit() * count;
    dst.store(item, count);
    src.mine(mass);

    MachineStatus::Running
}
