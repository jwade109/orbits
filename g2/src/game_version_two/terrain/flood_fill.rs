use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::game_version_two::CursorWorldPosition;

use super::constants::*;
use super::types::*;
use super::utils::*;

#[derive(Component, Debug)]
pub struct TerrainFloodFill {
    visited: HashSet<IVec2>,
    void_space: HashSet<IVec2>,
    open_set: HashSet<IVec2>,
    timer: Timer,
    ticks: usize,
    completed: usize,
}

impl TerrainFloodFill {
    fn new(global: IVec2) -> Self {
        dbg!(global);
        Self {
            visited: HashSet::new(),
            void_space: HashSet::new(),
            open_set: HashSet::from([global]),
            timer: Timer::from_seconds(0.02, TimerMode::Repeating),
            ticks: 0,
            completed: 0,
        }
    }
}

pub fn spawn_flood_fill(
    mut commands: Commands,
    cursor: Res<CursorWorldPosition>,
    btn: Res<ButtonInput<MouseButton>>,
) {
    let pos = match cursor.get() {
        Some(p) => p,
        _ => return,
    };

    if btn.just_pressed(MouseButton::Middle) {
        let (g, l) = to_grid_and_lattice(pos);
        let global = g * (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as i32 + l.as_ivec2();
        let flood = TerrainFloodFill::new(global);
        commands.spawn(flood);
    }
}

pub fn update_flood_fill(
    mut commands: Commands,
    mut flood: Query<(Entity, &mut TerrainFloodFill)>,
    chunks: Query<&TerrainChunk>,
    map: Res<ChunkMap>,
    time: Res<Time<Fixed>>,
) {
    for (e, mut flood) in flood {
        flood.timer.tick(time.delta());
        if !flood.timer.just_finished() {
            continue;
        }

        flood.ticks += 1;

        if flood.open_set.is_empty() {
            flood.completed += 1;
        }

        for _ in 0..10 {
            if let Some(gl) = flood.open_set.iter().next().cloned() {
                flood.visited.insert(gl);
                flood.open_set.remove(&gl);

                let (g, l) = global_to_gl(gl);
                if let Some(e) = map.get(&g) {
                    if let Ok(chunk) = chunks.get(*e) {
                        if !chunk.is_occupied(l) {
                            flood.void_space.insert(gl);
                            let left = gl - IVec2::X;
                            let right = gl + IVec2::X;
                            let bottom = gl - IVec2::Y;
                            let top = gl + IVec2::Y;

                            for n in [left, right, bottom, top] {
                                if !flood.visited.contains(&n) {
                                    flood.open_set.insert(n);
                                }
                            }
                        }
                    } else {
                        error!("Bad chunk!");
                    }
                }
            } else {
                break;
            }
        }

        if flood.open_set.is_empty() && flood.completed > 200 {
            commands.entity(e).despawn();
        }
    }
}

pub fn draw_flood_fill(mut painter: ShapePainter, flood: Query<&TerrainFloodFill>) {
    let w = CHUNK_WIDTH / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32 * 0.93;
    painter.reset();
    painter.set_color(GREEN.with_alpha(0.4));
    for flood in flood {
        for global in &flood.void_space {
            let pos = lattice_point_world_pos(IVec2::ZERO, *global);
            painter.set_translation(pos.extend(20.0));
            painter.rect(Vec2::splat(w));
        }
    }
}
