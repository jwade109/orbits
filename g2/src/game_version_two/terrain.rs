#![allow(unused)]

use crate::game_version_two::*;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkMap::default());
        app.add_event::<GenerateChunk>();
        app.add_event::<DeleteChunk>();
        app.add_event::<DensifyChunk>();
        app.add_systems(Startup, insert_tiles);
        app.add_systems(
            Update,
            (
                update_meshes,
                draw_hovered_grid,
                generate_tiles,
                delete_chunks,
                densify_chunks,
                throwaway_generate_chunks_input,
                draw_dense_lattice_values,
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
    dense: Option<DenseChunkData>,
}

const LATTICE_POINTS_PER_CHUNK_SIDE: usize = 10;

#[derive(Debug)]
pub struct DenseChunkData {
    points: [[f32; LATTICE_POINTS_PER_CHUNK_SIDE]; LATTICE_POINTS_PER_CHUNK_SIDE],
}

fn lattice_point_world_pos(g: IVec2, x: usize, y: usize) -> Vec2 {
    let buttom_left = g.as_vec2() * CHUNK_WIDTH - Vec2::splat(CHUNK_WIDTH) / 2.0;
    let xoff = (x as f32) / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32 * CHUNK_WIDTH;
    let yoff = (y as f32) / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32 * CHUNK_WIDTH;
    buttom_left + Vec2::new(xoff, yoff)
}

impl DenseChunkData {
    fn new(pos: IVec2) -> Self {
        let simplex = Simplex::new(1);

        let mut ret = Self {
            points: [[0.0; LATTICE_POINTS_PER_CHUNK_SIDE]; LATTICE_POINTS_PER_CHUNK_SIDE],
        };

        for x in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
            for y in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
                let p_world = lattice_point_world_pos(pos, x, y);
                let noise = simplex.get([p_world.x as f64 / 100.0, p_world.y as f64 / 100.0, 0.0]);
                ret.points[x][y] = noise as f32 + 0.5;
            }
        }

        ret
    }
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

#[derive(Event, Debug, Clone, Copy)]
pub struct DensifyChunk {
    pos: IVec2,
}

const CHUNK_WIDTH: f32 = 50.0;

fn insert_tiles(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image = Image::default();
    let handle = asset_server.add(image);
    commands.insert_resource(TerrainImage(handle));

    for x in -30..=30 {
        for y in -7..=0 {
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
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
            dense: None,
        };

        if msg.log {
            info!("Generating chunk {}: {:?}", msg.pos, chunk);
        }

        let transform = Transform::from_translation(msg.pos.as_vec2().extend(0.0) * CHUNK_WIDTH)
            .with_scale(Vec2::splat(CHUNK_WIDTH).extend(0.0));

        let mesh = Mesh2d(meshes.add(Rectangle::default()));
        let material = MeshMaterial2d(materials.add(Color::from(Srgba::gray(0.3))));

        let mut e = commands.spawn((chunk, transform, mesh, material));

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
        if btn.pressed(MouseButton::Middle) {
            let g = to_grid(p);
            commands.send_event(DensifyChunk { pos: g });
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

fn generate_mesh_data(chunk: &TerrainChunk) -> Mesh {
    if let Some(dense) = &chunk.dense {
        // let usage = RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD;
        // let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        // let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        // let uvs = vec![[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]];
        // let indices = vec![0, 1, 2];
        // let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, usage);
        // mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        // mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        // mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        // mesh.insert_indices(Indices::U32(indices));
        // mesh
        Rectangle::from_size(Vec2::splat(0.1)).into()
    } else {
        Rectangle::default().into()
    }
}

fn densify_chunks(
    mut chunks: Query<&mut TerrainChunk>,
    map: Res<ChunkMap>,
    mut messages: EventReader<DensifyChunk>,
) {
    for msg in messages.read() {
        if let Some(e) = map.get(&msg.pos) {
            if let Ok(mut chunk) = chunks.get_mut(*e) {
                chunk.dense = Some(DenseChunkData::new(chunk.pos));
            }
        } else {
            warn!("Failed to densify nonexistent chunk: {}", msg.pos);
        }
    }
}

fn update_meshes(
    mut chunks: Query<(&mut Mesh2d, &TerrainChunk)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mut mesh, mut chunk) in &mut chunks {
        let new_mesh = generate_mesh_data(&chunk);
        *mesh = meshes.add(new_mesh).into();
    }
}

fn draw_dense_lattice_values(
    mut painter: ShapePainter,
    chunks: Query<(&TerrainChunk, &GlobalTransform)>,
) {
    painter.reset();
    for (chunk, transform) in chunks {
        let origin = transform.translation().xy();
        if let Some(dense) = &chunk.dense {
            for x in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
                for y in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
                    let value = dense.points[x][y];
                    let u = x as f32 / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32;
                    let v = y as f32 / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32;
                    let color = Srgba::new(0.3 + u * 0.7, 0.3 + v * 0.7, 0.6, 1.0);
                    painter.set_color(color);
                    let p = lattice_point_world_pos(chunk.pos, x, y);
                    painter.set_translation(p.extend(10.0));
                    let w = CHUNK_WIDTH / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32;
                    painter.rect(Vec2::splat(value * w));
                }
            }
        }
    }
}
