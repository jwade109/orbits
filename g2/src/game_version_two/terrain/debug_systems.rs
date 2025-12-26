use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_vector_shapes::prelude::*;

use crate::game_version_two::{CursorWorldPosition, SelectedSpacecraft, Settings};

use super::constants::*;
use super::messages::*;
use super::terrain_helper::TerrainHelper;
use super::types::*;
use super::utils::to_grid_and_lattice;
use super::utils::*;

pub fn draw_hovered_grid_and_tile(
    mut painter: ShapePainter,
    terrain: TerrainHelper,
    cursor: Res<CursorWorldPosition>,
    settings: Res<Settings>,
) {
    let z = 120.0;
    let length = 3000.0;

    painter.reset();
    painter.set_color(RED);
    painter.line((Vec3::X * -length).with_z(z), (Vec3::X * length).with_z(z));
    painter.line((Vec3::Y * -length).with_z(z), (Vec3::Y * length).with_z(z));

    let cursor = match cursor.get() {
        Some(p) => p,
        _ => return,
    };

    let (g, l) = to_grid_and_lattice(cursor);

    if let Some(chunk) = terrain.chunk_at(cursor) {
        let center = chunk_center(g);

        painter.reset();
        painter.set_translation(center.extend(0.0));
        painter.hollow = true;
        painter.thickness = 2.0;
        painter.thickness_type = ThicknessType::Pixels;
        painter.rect(Vec2::splat(CHUNK_WIDTH));

        if settings.draw_terrain_rgb {
            painter.reset();
            if let Some(dense) = &chunk.dense {
                for x in 0..TILES_PER_CHUNK_SIDE {
                    for y in 0..TILES_PER_CHUNK_SIDE {
                        let tile = &dense.points[x][y];
                        let value = tile.mass.to_kg_f64().clamp(0.0, 1.0) as f32;
                        let u = x as f32 / TILES_PER_CHUNK_SIDE as f32;
                        let v = y as f32 / TILES_PER_CHUNK_SIDE as f32;
                        let color = Srgba::new(0.3 + u * 0.7, 0.3 + v * 0.7, 0.6, 0.6);
                        painter.set_color(color);
                        let l = IVec2::new(x as i32, y as i32);
                        let p = lattice_point_center_world_pos(chunk.pos, l);
                        painter.set_translation(p.extend(10.0));
                        let w = CHUNK_WIDTH / TILES_PER_CHUNK_SIDE as f32;
                        painter.rect(Vec2::splat(value * w));
                    }
                }
            }
        }
    }

    const Z_HOVER_TILE_DEBUG: f32 = 1.0;

    if let Some(tile) = terrain.chunk_at(cursor) {
        let center = lattice_point_center_world_pos(g, l.as_ivec2());

        painter.reset();
        painter.set_color(GREEN.with_alpha(0.8));
        painter.set_translation(center.extend(Z_HOVER_TILE_DEBUG));
        painter.hollow = true;
        painter.thickness = 2.0;
        painter.thickness_type = ThicknessType::Pixels;
        painter.rect(Vec2::splat(CHUNK_WIDTH / TILES_PER_CHUNK_SIDE as f32));
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

    let lattice_idx = (u * TILES_PER_CHUNK_SIDE as f32).round().as_ivec2();

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
        let center = ex.effector_center(tf);
        painter.set_translation(center.with_z(5.0));
        let color = if ex.is_on { GREEN } else { RED }.with_alpha(0.4);
        painter.set_color(color);
        painter.hollow = true;
        painter.circle(ex.radius);
    }
}

pub fn debug_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    cursor: Res<CursorWorldPosition>,
    terrain: TerrainHelper,
    settings: Res<Settings>,
) -> Result {
    if !settings.show_terrain_info {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    let Some(pos) = cursor.get_anyway() else {
        return Ok(());
    };

    let Some(chunk) = terrain.chunk_at(pos) else {
        return Ok(());
    };

    let (g, l) = to_grid_and_lattice(pos);

    egui::Window::new("Terrain Data").show(ctx, |ui| {
        crate::game_version_two::apply_egui_style(ui);
        ui.label(format!("Chunk pos: {}", g));
        ui.label(format!("Tile pos: {}", l));
        ui.label(format!("Global: {}", to_global(g, l.as_ivec2())));
        ui.label(format!("Is dense: {}", chunk.dense.is_some()));
        ui.label(format!("Mass: {}", chunk.mass()));

        let mut substrates: Vec<_> = chunk.substrates().into_iter().collect();
        substrates.sort();

        if substrates.is_empty() {
        } else {
            ui.label("Substrates:");
            for sub in substrates {
                let mass = chunk.mass_of(sub);
                ui.label(format!(" - {:?}: {}", sub, mass));
            }
        }

        if let Some(tile) = terrain.tile_at(pos) {
            ui.separator();
            ui.label(format!("Substrate: {:?}", tile.substrate));
            ui.label(format!("Mass: {}", tile.mass));
        }
    });

    Ok(())
}
