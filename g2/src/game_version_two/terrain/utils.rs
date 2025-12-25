use bevy::color::palettes::css::*;
use bevy::prelude::*;

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

    let lattice_idx = (u * (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32)
        .round()
        .as_uvec2();

    (g, lattice_idx)
}

pub fn chunk_bounds(g: IVec2) -> (Vec2, Vec2) {
    let lower = g.as_vec2() * CHUNK_WIDTH;
    let upper = lower + Vec2::splat(CHUNK_WIDTH);
    (lower, upper)
}

pub fn lattice_point_world_pos(g: IVec2, l: IVec2) -> Vec2 {
    let (lower, _) = chunk_bounds(g);
    let off = lattice_point_rel_pos(l);
    lower + off
}

pub fn lattice_point_rel_pos(l: IVec2) -> Vec2 {
    let xoff = (l.x as f32) / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32 * CHUNK_WIDTH;
    let yoff = (l.y as f32) / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32 * CHUNK_WIDTH;
    Vec2::new(xoff, yoff)
}

pub fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        return 0.0;
    }

    return (value - a) / (b - a);
}

fn marching_cubes_mesh(dense: &DenseChunkData) -> Mesh {
    let mut builder = MeshMaker::default();

    for (level, color) in [
        (0.5, Srgba::gray(DARK_COLOR)),
        (0.6, Srgba::gray(MEDIUM_COLOR)),
        (0.8, Srgba::gray(LIGHT_COLOR)),
    ] {
        builder.set_color(color);

        for x in 0..(LATTICE_POINTS_PER_CHUNK_SIDE - 1) {
            for y in 0..(LATTICE_POINTS_PER_CHUNK_SIDE - 1) {
                let value = dense.points[x][y];

                let bitmask = ((dense.points[x][y] > level) as u8)
                    | ((dense.points[x + 1][y] > level) as u8) << 1
                    | ((dense.points[x + 1][y + 1] > level) as u8) << 2
                    | ((dense.points[x][y + 1] > level) as u8) << 3;

                // let left = inverse_lerp(dense.points[x][y], dense.points[x][y + 1], 0.5);
                // let top = inverse_lerp(dense.points[x][y + 1], dense.points[x + 1][y + 1], 0.5);
                // let right = inverse_lerp(dense.points[x + 1][y], dense.points[x + 1][y + 1], 0.5);
                // let bottom = inverse_lerp(dense.points[x][y], dense.points[x + 1][y], 0.5);

                let square_size = 1.0 / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32 * CHUNK_WIDTH;

                let bottom_left = (IVec2::new(x as i32, y as i32).as_vec2()
                    / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32
                    - Vec2::splat(0.5))
                    * CHUNK_WIDTH;
                let top_right = bottom_left + Vec2::splat(square_size);
                let bottom_right = bottom_left + Vec2::X * square_size;
                let top_left = bottom_left + Vec2::Y * square_size;

                let lerp = false;

                let lerp_point = |vx: f32, vy: f32, px: Vec2, py: Vec2| {
                    let t = inverse_lerp(vx, vy, level);
                    px.lerp(py, t)
                };

                let (left, right, bottom, top) = if lerp {
                    let left = lerp_point(
                        dense.points[x][y],
                        dense.points[x][y + 1],
                        bottom_left,
                        top_left,
                    );
                    let right = lerp_point(
                        dense.points[x + 1][y],
                        dense.points[x + 1][y + 1],
                        bottom_right,
                        top_right,
                    );
                    let bottom = lerp_point(
                        dense.points[x][y],
                        dense.points[x + 1][y],
                        bottom_left,
                        bottom_right,
                    );
                    let top = lerp_point(
                        dense.points[x][y + 1],
                        dense.points[x + 1][y + 1],
                        top_left,
                        top_right,
                    );
                    (left, right, bottom, top)
                } else {
                    let left = bottom_left + Vec2::Y * square_size * 0.5;
                    let bottom = bottom_left + Vec2::X * square_size * 0.5;
                    let right = top_right - Vec2::Y * square_size * 0.5;
                    let top = top_right - Vec2::X * square_size * 0.5;
                    (left, right, bottom, top)
                };

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

pub fn generate_mesh_data(chunk: &TerrainChunk) -> (Mesh, Srgba) {
    if let Some(dense) = &chunk.dense {
        if dense.is_empty() {
            (Rectangle::new(0.1, 0.1).into(), RED)
        } else {
            (marching_cubes_mesh(dense), WHITE)
        }
    } else {
        (Rectangle::from_size(Vec2::splat(CHUNK_WIDTH)).into(), WHITE)
    }
}

pub fn global_to_gl(gl: IVec2) -> (IVec2, IVec2) {
    let n = (LATTICE_POINTS_PER_CHUNK_SIDE - 1);
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
