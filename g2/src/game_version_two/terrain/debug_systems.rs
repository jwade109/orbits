use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;

use crate::game_version_two::{CursorWorldPosition, SelectedSpacecraft, Settings};

use super::constants::*;
use super::messages::*;
use super::types::*;
use super::utils::*;

pub fn draw_hovered_grid(
    mut painter: ShapePainter,
    chunks: Query<(&GlobalTransform, &TerrainChunk)>,
    map: Res<ChunkMap>,
    cursor: Res<CursorWorldPosition>,
    settings: Res<Settings>,
) {
    let z = 120.0;
    let length = 3000.0;

    painter.reset();
    painter.set_color(RED);
    painter.line((Vec3::X * -length).with_z(z), (Vec3::X * length).with_z(z));
    painter.line((Vec3::Y * -length).with_z(z), (Vec3::Y * length).with_z(z));

    let cursor: Vec2 = match cursor.get() {
        Some(p) => p,
        _ => return,
    };

    if let Some(e) = map.lup(cursor) {
        if let Ok((tf, chunk)) = chunks.get(e) {
            painter.reset();
            painter.set_translation(tf.translation());
            painter.hollow = true;
            painter.thickness = 2.0;
            painter.thickness_type = ThicknessType::Pixels;
            painter.rect(Vec2::splat(CHUNK_WIDTH));

            if settings.draw_terrain_rgb {
                painter.reset();
                if let Some(dense) = &chunk.dense {
                    for x in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
                        for y in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
                            let value = dense.points[x][y];
                            let u = x as f32 / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32;
                            let v = y as f32 / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32;
                            let color = Srgba::new(0.3 + u * 0.7, 0.3 + v * 0.7, 0.6, 0.6);
                            painter.set_color(color);
                            let l = IVec2::new(x as i32, y as i32);
                            let p = lattice_point_world_pos(chunk.pos, l);
                            painter.set_translation(p.extend(10.0));
                            let w = CHUNK_WIDTH / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32;
                            painter.rect(Vec2::splat(value * w));
                        }
                    }
                }
            }
        }
    }
}

pub fn draw_highlighted_lattice_points(
    mut commands: Commands,
    mut painter: ShapePainter,
    map: Res<ChunkMap>,
    cursor: Res<CursorWorldPosition>,
    btn: Res<ButtonInput<MouseButton>>,
    info: Res<SelectedSpacecraft>,
    settings: Res<Settings>,
) {
    let pos = match cursor.get() {
        Some(p) => p,
        _ => return,
    };

    let g = to_grid(pos);
    let (lower, upper) = chunk_bounds(g);
    let u = (pos - lower) / CHUNK_WIDTH;

    let lattice_idx = (u * (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32)
        .round()
        .as_ivec2();

    let (x, y) = (lattice_idx.x as usize, lattice_idx.y as usize);

    if info.hovered.is_none() {
        if btn.pressed(MouseButton::Left) && settings.dig_with_mouse {
            let dig = Excavate {
                pos,
                radius: 12.0,
                is_fill: false,
            };
            commands.send_event(dig);
        }
        if btn.pressed(MouseButton::Right) && settings.dig_with_mouse {
            let dig = Excavate {
                pos,
                radius: 12.0,
                is_fill: true,
            };
            commands.send_event(dig);
        }
    }
}

pub fn draw_excavators(
    mut painter: ShapePainter,
    excavators: Query<(&GlobalTransform, &Excavator)>,
) {
    for (tf, ex) in excavators {
        painter.reset();
        painter.set_translation(tf.translation().with_z(60.0));
        let color = if ex.is_enabled { GREEN } else { RED }.with_alpha(0.4);
        painter.set_color(color);
        painter.hollow = true;
        painter.circle(ex.radius);
    }
}
