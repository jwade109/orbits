#![allow(unused)]

use crate::game_version_two::*;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkMap::default());
        app.add_event::<GenerateChunk>();
        app.add_event::<DeleteChunk>();
        app.add_systems(Startup, insert_tiles);
        app.add_systems(
            Update,
            (
                draw_hovered_grid,
                generate_tiles,
                delete_chunks,
                throwaway_generate_chunks_input,
            ),
        );
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct ChunkMap(HashMap<IVec2, Entity>);

pub fn to_grid(pos: Vec2) -> IVec2 {
    (pos / CHUNK_WIDTH).round().as_ivec2()
}

pub fn chunk_bounds(g: IVec2) -> (Vec2, Vec2) {
    let p = g.as_vec2() * CHUNK_WIDTH;
    let q = (g + IVec2::ONE).as_vec2() * CHUNK_WIDTH;
    (p, q)
}

impl ChunkMap {
    pub fn lup(&self, pos: Vec2) -> Option<Entity> {
        let g = to_grid(pos);
        self.0.get(&g).cloned()
    }
}

#[derive(Resource)]
struct TerrainImage(Handle<Image>);

#[derive(Component, Debug)]
pub struct TerrainChunk {
    pos: IVec2,
    contents: Item,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct GenerateChunk {
    pos: IVec2,
    material: Option<Item>,
    log: bool,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct DeleteChunk {
    pos: IVec2,
    log: bool,
}

const CHUNK_WIDTH: f32 = 5.0;

fn insert_tiles(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image = Image::default();
    let handle = asset_server.add(image);
    commands.insert_resource(TerrainImage(handle));

    for x in -300..=300 {
        let height = ((x as f32 / 14.0).sin() * 8.0).round() as i32;
        for y in -50..=height {
            commands.send_event(GenerateChunk {
                pos: IVec2::new(x, y),
                material: None,
                log: false,
            });
        }
    }
}

fn generate_tiles(
    mut commands: Commands,
    mut messages: EventReader<GenerateChunk>,
    mut chunk_map: ResMut<ChunkMap>,
    terrain_image: Res<TerrainImage>,
) {
    for msg in messages.read() {
        if chunk_map.contains_key(&msg.pos) {
            warn!("Chunk already generated: {}", msg.pos);
            continue;
        }

        let mut s = Sprite::from_image(terrain_image.0.clone());
        let contents = chance(0.02)
            .then(|| Item::random_mineable())
            .unwrap_or(Item::Stone);

        let contents = msg.material.unwrap_or(contents);

        s.color = contents.color().into();

        let chunk = TerrainChunk {
            pos: msg.pos,
            contents,
        };

        if msg.log {
            info!("Generating chunk {}: {:?}", msg.pos, chunk);
        }

        let transform = Transform::from_translation(msg.pos.as_vec2().extend(0.0) * CHUNK_WIDTH)
            .with_scale(Vec2::splat(CHUNK_WIDTH).extend(0.0));

        let mut e = commands.spawn((s.clone(), chunk, transform));

        let vol = Volume::liters(randint(1000, 10000) as u64);
        let mut inv = Inventory::single(contents, vol);
        inv.fill();
        e.insert(inv);

        chunk_map.insert(msg.pos, e.id());
    }
}

fn delete_chunks(
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
        } else {
            warn!("Chunk to delete does not exist: {}", msg.pos);
        }
        chunk_map.remove(&msg.pos);
    }
}

fn throwaway_generate_chunks_input(
    mut commands: Commands,
    mouse: Res<CursorWorldPosition>,
    mut sel: ResMut<CursorInfo>,
    btn: Res<ButtonInput<MouseButton>>,
    map: Res<ChunkMap>,
) {
    if let Some(p) = mouse.get() {
        sel.hovered = map.lup(p);
        if btn.pressed(MouseButton::Left) {
            let g = to_grid(p);
            commands.send_event(GenerateChunk {
                pos: g,
                material: Some(Item::Concrete),
                log: false,
            });
        }
        if btn.pressed(MouseButton::Right) {
            let g = to_grid(p);
            commands.send_event(DeleteChunk { pos: g, log: false });
        }
    }
}

fn draw_hovered_grid(
    mut painter: ShapePainter,
    chunks: Query<&GlobalTransform, With<TerrainChunk>>,
    map: Res<ChunkMap>,
    cursor: Res<CursorWorldPosition>,
) {
    let cursor: Vec2 = match cursor.get() {
        Some(p) => p,
        _ => return,
    };

    if let Some(e) = map.lup(cursor) {
        if let Ok(tf) = chunks.get(e) {
            painter.reset();
            painter.set_translation(tf.translation());
            painter.hollow = true;
            painter.thickness = 2.0;
            painter.thickness_type = ThicknessType::Pixels;
            painter.rect(Vec2::splat(CHUNK_WIDTH));
        }
    }
}
