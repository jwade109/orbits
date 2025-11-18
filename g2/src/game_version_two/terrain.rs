#![allow(unused)]

use crate::game_version_two::*;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkMap::default());
        app.add_event::<GenerateChunk>();
        app.add_event::<DeleteChunk>();
        app.add_event::<Excavate>();
        app.add_systems(Startup, insert_tiles);
        app.add_systems(
            Update,
            (
                update_meshes,
                draw_hovered_grid,
                generate_tiles,
                delete_chunks,
                draw_highlighted_lattice_points,
                excavate_chunks,
                process_excavators,
                draw_excavators,
                spawn_flood_fill,
                update_flood_fill,
                draw_flood_fill,
            ),
        );
    }
}

pub fn to_grid(pos: Vec2) -> IVec2 {
    (pos / CHUNK_WIDTH).round().as_ivec2()
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
    let p = g.as_vec2() * CHUNK_WIDTH - Vec2::splat(CHUNK_WIDTH) / 2.0;
    let q = (g + IVec2::ONE).as_vec2() * CHUNK_WIDTH;
    (p, q)
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct ChunkMap(HashMap<IVec2, Entity>);

impl ChunkMap {
    pub fn lup(&self, pos: Vec2) -> Option<Entity> {
        let g = to_grid(pos);
        self.0.get(&g).cloned()
    }
}

#[derive(Component, Debug)]
pub struct TerrainChunk {
    pos: IVec2,
    dense: Option<DenseChunkData>,
    needs_mesh_update: bool,
}

impl TerrainChunk {
    fn is_empty(&self) -> bool {
        self.dense.as_ref().map(|d| d.is_empty()).unwrap_or(true)
    }

    fn is_occupied(&self, l: IVec2) -> bool {
        if l.x < 0
            || l.x >= LATTICE_POINTS_PER_CHUNK_SIDE as i32
            || l.y < 0
            || l.y >= LATTICE_POINTS_PER_CHUNK_SIDE as i32
        {
            return true;
        }
        let dense = match &self.dense {
            Some(d) => d,
            _ => return true,
        };
        let x = l.x as usize;
        let y = l.y as usize;

        dense.points[x][y] > 0.5
    }
}

const LATTICE_POINTS_PER_CHUNK_SIDE: usize = 20;

fn lattice_point_world_pos(g: IVec2, l: IVec2) -> Vec2 {
    let buttom_left = g.as_vec2() * CHUNK_WIDTH - Vec2::splat(CHUNK_WIDTH) / 2.0;
    let off = lattice_point_rel_pos(l);
    buttom_left + off
}

fn lattice_point_rel_pos(l: IVec2) -> Vec2 {
    let xoff = (l.x as f32) / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32 * CHUNK_WIDTH;
    let yoff = (l.y as f32) / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32 * CHUNK_WIDTH;
    Vec2::new(xoff, yoff)
}

#[derive(Debug)]
pub struct DenseChunkData {
    points: [[f32; LATTICE_POINTS_PER_CHUNK_SIDE]; LATTICE_POINTS_PER_CHUNK_SIDE],
}

impl DenseChunkData {
    fn new(pos: IVec2, z: f32) -> Self {
        let simplex = Simplex::new(1);

        let mut ret = Self {
            points: [[0.0; LATTICE_POINTS_PER_CHUNK_SIDE]; LATTICE_POINTS_PER_CHUNK_SIDE],
        };

        for x in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
            for y in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
                let p_world = lattice_point_world_pos(pos, IVec2::new(x as i32, y as i32));
                let noise =
                    simplex.get([p_world.x as f64 / 100.0, p_world.y as f64 / 100.0, z as f64]);
                ret.points[x][y] = (noise as f32 + 0.5) * 0.3 + 0.7;
            }
        }

        ret
    }

    fn solid() -> Self {
        Self {
            points: [[1.0; LATTICE_POINTS_PER_CHUNK_SIDE]; LATTICE_POINTS_PER_CHUNK_SIDE],
        }
    }

    fn is_solid(&self) -> bool {
        self.points.iter().all(|arr| arr.iter().all(|x| *x > 0.8))
    }

    fn is_empty(&self) -> bool {
        self.points.iter().all(|arr| arr.iter().all(|x| *x < 0.5))
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
pub struct Excavate {
    pos: Vec2,
    radius: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Excavator {
    pub is_enabled: bool,
    pub radius: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct OreDeposit;

const CHUNK_WIDTH: f32 = 50.0;

fn insert_tiles(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image = Image::default();
    let handle = asset_server.add(image);

    for x in -30..=30 {
        for y in -7..12 {
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

        let transform = Transform::from_translation(msg.pos.as_vec2().extend(0.0) * CHUNK_WIDTH);

        if chance(0.02) {
            let item = Item::random_mineable();
            let n = randint(50, 80);
            for _ in 0..n {
                let offset = randvec(0.1, CHUNK_WIDTH);
                let transform = Transform::from_translation(
                    (msg.pos.as_vec2() * CHUNK_WIDTH + offset).extend(-5.0),
                );
                let capacity = randint(2000, 200000) as u64 * item.volume_per_unit();
                let ore = Inventory::single(item, capacity);
                let r = rand(4.0, 9.0);
                let angle = rand(0.0, std::f32::consts::PI * 2.0);
                let a = Vec2::from_angle(angle) * r;
                let b = Vec2::from_angle(angle + std::f32::consts::PI * 2.0 / 3.0) * r;
                let c = Vec2::from_angle(angle + std::f32::consts::PI * 4.0 / 3.0) * r;
                let mesh = Mesh2d(meshes.add(Triangle2d::new(a, b, c)));
                let mut color = item.color();
                color.red = (color.red + rand(-0.04, 0.04)).clamp(0.0, 1.0);
                color.green = (color.green + rand(-0.04, 0.04)).clamp(0.0, 1.0);
                color.blue = (color.blue + rand(-0.04, 0.04)).clamp(0.0, 1.0);
                let material = MeshMaterial2d(materials.add(Color::from(color)));
                commands.spawn((ore, transform, OreDeposit, mesh, material));
            }
        }

        let bg_mesh = Mesh2d(meshes.add(Rectangle::from_length(CHUNK_WIDTH)));
        let bg_material = MeshMaterial2d(materials.add(Color::from(Srgba::gray(VERY_DARK_COLOR))));
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
        }
        chunk_map.remove(&msg.pos);
    }
}

fn draw_hovered_grid(
    mut painter: ShapePainter,
    chunks: Query<(&GlobalTransform, &TerrainChunk)>,
    map: Res<ChunkMap>,
    cursor: Res<CursorWorldPosition>,
) {
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

                        // painter.set_color(WHITE);
                        // painter.circle(1.1);
                        // painter.set_color(BLACK);
                        // painter.circle(1.05);
                        // painter.set_color(WHITE);
                        // painter.circle(1.0 * value);
                    }
                }
            }
        }
    }
}

fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        return 0.0;
    }

    return (value - a) / (b - a);
}

const VERY_DARK_COLOR: f32 = 0.1;
const DARK_COLOR: f32 = 0.15;
const MEDIUM_COLOR: f32 = 0.25;
const LIGHT_COLOR: f32 = 0.3;

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

                let lerp = true;

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

fn generate_mesh_data(chunk: &TerrainChunk) -> (Mesh, Srgba) {
    if let Some(dense) = &chunk.dense {
        if dense.is_empty() {
            (Rectangle::new(0.1, 0.1).into(), RED)
        } else if dense.is_solid() {
            (
                Rectangle::from_size(Vec2::splat(CHUNK_WIDTH)).into(),
                Srgba::gray(LIGHT_COLOR),
            )
        } else {
            (marching_cubes_mesh(dense), WHITE)
        }
    } else {
        (Rectangle::from_size(Vec2::splat(CHUNK_WIDTH)).into(), WHITE)
    }
}

fn update_meshes(
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

fn excavate_chunks(
    map: Res<ChunkMap>,
    mut messages: EventReader<Excavate>,
    mut chunks: Query<&mut TerrainChunk>,
) {
    for dig in messages.read() {
        let center = to_grid(dig.pos);

        for xi in center.x - 1..=center.x + 1 {
            for yi in center.y - 1..=center.y + 1 {
                let g = IVec2::new(xi, yi);
                let chunk_id = match map.get(&g) {
                    Some(id) => id,
                    _ => continue,
                };

                let mut chunk = match chunks.get_mut(*chunk_id) {
                    Ok(mut chunk) => chunk,
                    _ => continue,
                };

                let dense = match &mut chunk.dense {
                    Some(dense) => dense,
                    _ => continue,
                };

                let mut needs_mesh_update = false;

                for x in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
                    for y in 0..LATTICE_POINTS_PER_CHUNK_SIDE {
                        let world_pos = lattice_point_world_pos(g, IVec2::new(x as i32, y as i32));
                        let d = world_pos.distance(dig.pos);
                        let value = (0.5 * d / dig.radius).clamp(0.0, 1.0);
                        if dense.points[x][y] > value {
                            dense.points[x][y] += (value - dense.points[x][y]) * 0.1;
                            needs_mesh_update = true;
                        }
                    }
                }

                chunk.needs_mesh_update |= needs_mesh_update;
            }
        }
    }
}

fn draw_highlighted_lattice_points(
    mut commands: Commands,
    mut painter: ShapePainter,
    map: Res<ChunkMap>,
    cursor: Res<CursorWorldPosition>,
    mut chunks: Query<&mut TerrainChunk>,
    btn: Res<ButtonInput<MouseButton>>,
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

    let world = lattice_point_world_pos(g, lattice_idx);

    painter.reset();
    painter.set_translation(world.extend(100.0));
    painter.circle(0.1);

    if btn.pressed(MouseButton::Left) {
        let dig = Excavate { pos, radius: 12.0 };
        commands.send_event(dig);
    }
}

fn process_excavators(
    mut events: EventWriter<Excavate>,
    excavators: Query<(&GlobalTransform, &Excavator)>,
) {
    for (tf, ex) in excavators {
        if !ex.is_enabled {
            continue;
        }

        let pos = tf.translation().xy();
        let msg = Excavate {
            pos,
            radius: ex.radius,
        };
        events.write(msg);
    }
}

fn draw_excavators(mut painter: ShapePainter, excavators: Query<(&GlobalTransform, &Excavator)>) {
    for (tf, ex) in excavators {
        painter.reset();
        painter.set_translation(tf.translation().with_z(60.0));
        let color = if ex.is_enabled { GREEN } else { RED }.with_alpha(0.4);
        painter.set_color(color);
        painter.hollow = true;
        painter.circle(ex.radius);
    }
}

#[derive(Component, Debug)]
pub struct TerrainFloodFill {
    visited: HashSet<IVec2>,
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
            open_set: HashSet::from([global]),
            timer: Timer::from_seconds(0.02, TimerMode::Repeating),
            ticks: 0,
            completed: 0,
        }
    }
}

fn spawn_flood_fill(
    mut commands: Commands,
    cursor: Res<CursorWorldPosition>,
    btn: Res<ButtonInput<MouseButton>>,
) {
    let pos = match cursor.get() {
        Some(p) => p,
        _ => return,
    };

    if btn.just_pressed(MouseButton::Right) {
        let (g, l) = to_grid_and_lattice(pos);
        let global = g * (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as i32 + l.as_ivec2();
        let flood = TerrainFloodFill::new(global);
        commands.spawn(flood);
    }
}

fn global_to_gl(gl: IVec2) -> (IVec2, IVec2) {
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

fn update_flood_fill(
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
                println!("{:?}", (g, l));
                if let Some(e) = map.get(&g) {
                    if let Ok(chunk) = chunks.get(*e) {
                        if !chunk.is_occupied(l) {
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
                        println!("Bad chunk!");
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

fn draw_flood_fill(mut painter: ShapePainter, flood: Query<&TerrainFloodFill>) {
    let w = CHUNK_WIDTH / (LATTICE_POINTS_PER_CHUNK_SIDE - 1) as f32 * 0.93;
    painter.reset();
    painter.set_color(GREEN.with_alpha(0.4));
    for flood in flood {
        for global in &flood.visited {
            let pos = lattice_point_world_pos(IVec2::ZERO, *global);
            painter.set_translation(pos.extend(20.0));
            painter.rect(Vec2::splat(w));
        }
    }
}
