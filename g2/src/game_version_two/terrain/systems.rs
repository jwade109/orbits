use bevy::prelude::*;
use early_returns::ok_or_continue;
use early_returns::some_or_continue;
use game::starling::prelude::*;

use super::constants::*;
use super::messages::*;
use super::types::*;
use super::utils::*;

/// startup system to spawn new tiles
pub fn insert_tiles(mut commands: Commands, asset_server: Res<AssetServer>) {
    for x in -30..=30 {
        for y in -30..=30 {
            let r = Vec2::new(x as f32, y as f32);
            if r.length() > 12.8 {
                continue;
            }
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
) {
    for msg in messages.read() {
        if chunk_map.contains_key(&msg.pos) {
            continue;
        }

        let contents = chance(0.02)
            .then(|| Item::random_mineable())
            .unwrap_or(Item::Stone);

        let contents = msg.material.unwrap_or(contents);

        let chunk = TerrainChunk {
            pos: msg.pos,
            dense: Some(DenseChunkData::new(msg.pos, 0.0)),
            needs_mesh_update: true,
        };

        if msg.log {
            info!("Generating chunk {}: {:?}", msg.pos, chunk);
        }

        let transform = Transform::from_translation(
            (msg.pos.as_vec2() * CHUNK_WIDTH + Vec2::splat(CHUNK_WIDTH / 2.0)).extend(0.0),
        );

        let bg_mesh = Mesh2d(meshes.add(Rectangle::from_length(CHUNK_WIDTH)));
        let bg_material = MeshMaterial2d(materials.add(Color::from(Srgba::gray(0.2))));
        let child = commands
            .spawn((
                bg_mesh,
                bg_material,
                Visibility::Visible,
                Transform::from_xyz(0.0, 0.0, -10.0),
            ))
            .id();

        let mesh = Mesh2d(meshes.add(Rectangle::from_length(CHUNK_WIDTH)));
        let material = MeshMaterial2d(materials.add(Color::default()));

        let mut e = commands.spawn((chunk, transform, mesh, material));

        e.add_child(child);

        let vol = Volume::liters(randint(1000, 10000) as u64);
        let mut inv = Inventory::single(contents, vol);
        inv.fill();
        e.insert(inv);

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

        chunk.needs_mesh_update |= tile.mine(delta);

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
        if !ex.is_enabled {
            continue;
        }

        ex.timer.tick(time.delta());

        if ex.timer.just_finished() {
            let pos = tf.translation().xy();
            let (g, l) = to_grid_and_lattice(pos);
            let msg = MineToInventory {
                chunk: g,
                tile: l,
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
    mut inventories: Query<&mut Inventory>,
) {
    for event in events.read() {
        info!(?event);

        let Ok(mut inv) = inventories.get_mut(event.inventory) else {
            error!("Failed to get inventory: {}", event.inventory);
            continue;
        };

        let e = some_or_continue!(chunk_map.get(&event.chunk));
        let mut chunk = ok_or_continue!(chunks.get_mut(*e));
        let mut dense = some_or_continue!(chunk.dense.as_mut());
        let mut tile = &mut dense.points[event.tile.x as usize][event.tile.y as usize];

        inv.store(tile.substrate.yields(), 1000);

        chunk.needs_mesh_update |= tile.mine(Mass::kilograms(150));
    }
}
