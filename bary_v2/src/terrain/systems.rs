use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use early_returns::ok_or_continue;
use early_returns::some_or_continue;
use bary_core::prelude::*;

use super::constants::*;
use super::messages::*;
use super::types::*;
use super::utils::*;

/// startup system to spawn new tiles
pub fn insert_tiles(mut commands: Commands, asset_server: Res<AssetServer>) {
    let ast = Asteroid::random(600.0, Some(12));

    commands.insert_resource(Ast(ast));

    for x in -30..=30 {
        for y in -30..=30 {
            commands.send_event(GenerateChunk {
                pos: IVec2::new(x, y),
                material: None,
                log: false,
            });
        }
    }
}

pub fn generate_tiles(
    mut commands: Commands,
    mut messages: EventReader<GenerateChunk>,
    mut chunk_map: ResMut<ChunkMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asteroid: Res<Ast>,
) {
    for msg in messages.read() {
        if chunk_map.contains_key(&msg.pos) {
            continue;
        }

        let chunk = TerrainChunk {
            pos: msg.pos,
            dense: Some(DenseChunkData::new(msg.pos, 0.0, &asteroid)),
            needs_mesh_update: true,
        };

        if msg.log {
            info!("Generating chunk {}: {:?}", msg.pos, chunk);
        }

        let transform = Transform::from_translation(
            (msg.pos.as_vec2() * CHUNK_WIDTH + Vec2::splat(CHUNK_WIDTH / 2.0)).extend(0.0),
        );

        let mesh = Mesh2d(meshes.add(Rectangle::from_length(CHUNK_WIDTH)));
        let material = MeshMaterial2d(materials.add(Color::default()));

        let mut e = commands.spawn((chunk, transform, mesh, material));

        chunk_map.insert(msg.pos, e.id());
    }
}

pub fn delete_chunks(
    mut commands: Commands,
    mut messages: EventReader<DeleteChunk>,
    mut chunk_map: ResMut<ChunkMap>,
) {
    for msg in messages.read() {
        if let Some(e) = chunk_map.get(&msg.pos) {
            commands.entity(*e).despawn();
            if msg.log {
                info!("Deleting chunk {}", msg.pos);
            }
        }
        chunk_map.remove(&msg.pos);
    }
}

pub fn update_meshes(
    mut chunks: Query<
        (
            &mut Visibility,
            &mut Mesh2d,
            &mut TerrainChunk,
            &mut MeshMaterial2d<ColorMaterial>,
        ),
        Changed<TerrainChunk>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (mut vis, mut mesh, mut chunk, mut mat) in &mut chunks {
        if !chunk.needs_mesh_update {
            continue;
        }
        let (new_mesh, color) = generate_mesh_data(&chunk);
        *vis = if chunk.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        *mat = MeshMaterial2d(materials.add(Color::from(color)));
        *mesh = meshes.add(new_mesh).into();
        chunk.needs_mesh_update = false;
    }
}

pub fn excavate_chunks(
    map: Res<ChunkMap>,
    mut messages: EventReader<Excavate>,
    mut chunks: Query<&mut TerrainChunk>,
) {
    for dig in messages.read() {
        let (g, l) = to_grid_and_lattice(dig.pos);

        let e = some_or_continue!(map.get(&g));
        let mut chunk = ok_or_continue!(chunks.get_mut(*e));
        let dense = some_or_continue!(chunk.dense.as_mut());
        let tile = &mut dense.points[l.x as usize][l.y as usize];

        let delta = Mass::kilograms(150);

        tile.mine(delta);

        chunk.needs_mesh_update = true;

        // for xi in center.x - 1..=center.x + 1 {
        //     for yi in center.y - 1..=center.y + 1 {
        //         let g = IVec2::new(xi, yi);
        //         let chunk_id = match map.get(&g) {
        //             Some(id) => id,
        //             _ => continue,
        //         };

        //         let mut chunk = match chunks.get_mut(*chunk_id) {
        //             Ok(mut chunk) => chunk,
        //             _ => continue,
        //         };

        //         let dense = match &mut chunk.dense {
        //             Some(dense) => dense,
        //             _ => continue,
        //         };

        //         let mut needs_mesh_update = false;

        //         // for x in 0..TILES_PER_CHUNK_SIDE {
        //         //     for y in 0..TILES_PER_CHUNK_SIDE {
        //         //         let world_pos =
        //         //             lattice_point_center_world_pos(g, IVec2::new(x as i32, y as i32));
        //         //         let d = world_pos.distance(dig.pos);
        //         //         let (value, trigger) = if dig.is_fill {
        //         //             let value = (1.0 - (0.5 * d / dig.radius)).clamp(0.0, 1.0);
        //         //             (value, dense.points[x][y] < value)
        //         //         } else {
        //         //             let value = (0.5 * d / dig.radius).clamp(0.0, 1.0);
        //         //             (value, dense.points[x][y] > value)
        //         //         };
        //         //         if trigger {
        //         //             let delta_mass_kg =
        //         //                 (value - dense.points[x][y].mass.to_kg_f64() as f32) * 0.1;
        //         //             dense.points[x][y].apply_delta_mass(delta_mass_kg);
        //         //             needs_mesh_update = true;
        //         //         }
        //         //     }
        //         // }

        //         chunk.needs_mesh_update |= needs_mesh_update;
        //     }
        // }
    }
}

pub fn process_excavators(
    mut events: EventWriter<MineToInventory>,
    excavators: Query<(Entity, &GlobalTransform, &mut Excavator)>,
    time: Res<Time<Fixed>>,
) {
    for (e, tf, mut ex) in excavators {
        if !ex.is_on {
            ex.status = MachineStatus::Off;
            ex.last_op_status = MachineStatus::Off;
            continue;
        }

        ex.status = MachineStatus::Running;

        ex.timer.tick(time.delta());

        if ex.timer.just_finished() {
            let pos = ex.effector_center(tf).xy();
            let offset = randvec(0.0, ex.radius);
            let (g, l) = to_grid_and_lattice(pos);
            let msg = MineToInventory {
                pos: pos + offset,
                inventory: e,
            };
            events.write(msg);
        }
    }
}

pub fn process_mine_to_inventory(
    mut events: EventReader<MineToInventory>,
    chunk_map: Res<ChunkMap>,
    mut chunks: Query<&mut TerrainChunk>,
    mut excavators: Query<(&mut Inventory, &mut Excavator)>,
) -> Vec<(Vec2, MiningFailure)> {
    let mut successes = Vec::new();

    for event in events.read() {
        let (chunk, tile) = to_grid_and_lattice(event.pos);

        let Ok((mut inv, mut ex)) = excavators.get_mut(event.inventory) else {
            error!("Failed to get inventory: {}", event.inventory);
            continue;
        };

        let e = some_or_continue!(chunk_map.get(&chunk));
        let mut chunk = ok_or_continue!(chunks.get_mut(*e));
        let mut dense = some_or_continue!(chunk.dense.as_mut());
        let mut tile = &mut dense.points[tile.x as usize][tile.y as usize];

        if tile.mass.is_zero() {
            successes.push((event.pos, MiningFailure::NoMaterial));
            ex.last_op_status = MachineStatus::Starved;
            continue;
        }

        let old_mass = tile.mass;

        let count = 10000;
        let item = tile.substrate.yields();
        let mass = item.mass_per_unit() * count;

        ex.last_op_status = atomic_mine(&mut tile, &mut inv, item, count);

        let status = if ex.last_op_status == MachineStatus::Running {
            MiningFailure::Ok
        } else {
            MiningFailure::NoRoom
        };

        successes.push((event.pos, status));

        if tile.mass != old_mass {
            chunk.needs_mesh_update = true;
        }
    }

    successes
}

pub fn spawn_mining_visuals(In(chunks): In<Vec<(Vec2, MiningFailure)>>, mut commands: Commands) {
    for (pos, success) in chunks {
        let indicator = MiningIndicator {
            remaining: Timer::from_seconds(1.0, TimerMode::Once),
            pos,
            success,
        };
        commands.spawn(indicator);
    }
}

pub fn age_mining_visuals_system(
    mut commands: Commands,
    indicators: Query<(Entity, &mut MiningIndicator)>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta();
    for (e, mut ind) in indicators {
        ind.remaining.tick(dt);
        if ind.remaining.finished() {
            commands.entity(e).despawn();
        }
    }
}

pub fn render_mining_indicators_system(mut painter: ShapePainter, indicators: Query<&MiningIndicator>) {
    painter.reset();
    let side_length = CHUNK_WIDTH / TILES_PER_CHUNK_SIDE as f32 / 4.0;
    for ind in indicators {
        let remaining = ind.remaining.remaining().as_secs_f32();
        let color = match ind.success {
            MiningFailure::Ok => ORANGE,
            MiningFailure::NoRoom => RED,
            MiningFailure::NoMaterial => TEAL,
        }
        .with_alpha(remaining * 0.5 + 0.5);
        painter.set_color(color);
        painter.set_translation(ind.pos.extend(3.0));
        painter.rect(Vec2::splat(side_length));
    }
}
