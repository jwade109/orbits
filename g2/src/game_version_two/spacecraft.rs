use crate::game_version_two::*;

use avian2d::prelude::{AngularVelocity, Collider, PhysicsPlugins};
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_ecs::relationship::RelatedSpawnerCommands;
use bevy_vector_shapes::prelude::*;
use game::args::ProgramContext;
use starling::prelude::{InstantiatedPart, InstantiatedPartVariant, Vehicle, rand};

pub struct SpacecraftPlugin;

impl Plugin for SpacecraftPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default());
        app.insert_state(DebugGrids::Drawn);
        app.insert_state(DebugInventories::Drawn);

        app.add_systems(
            Update,
            (
                draw_grids.run_if(in_state(DebugGrids::Drawn)),
                draw_inventories.run_if(in_state(DebugInventories::Drawn)),
                handle_sc_events,
                handle_change_recipe,
                draw_selected_part,
                draw_selected_grid_guides,
            ),
        );

        app.add_systems(
            FixedUpdate,
            (
                build_parts,
                update_machines,
                accelerate_spacecraft,
                update_grids,
            ),
        );

        app.add_event::<SpacecraftEvent>();
        app.add_event::<SetRecipe>();

        app.insert_resource(CursorInfo::default());
    }
}

#[derive(States, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum DebugGrids {
    Hidden,
    Drawn,
}

#[derive(States, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum DebugInventories {
    Hidden,
    Drawn,
}

#[derive(Resource, Debug, Default)]
pub struct CursorInfo {
    pub selected: Option<Entity>,
    pub hovered: Option<Entity>,
}

#[derive(Event, Debug)]
pub enum SpacecraftEvent {
    SpawnVehicle { name: String, pos: Vec2, angle: f32 },
    SpawnPart { name: String, pos: Vec2, angle: f32 },
    Destroy { target: Entity },
}

#[derive(Event, Debug)]
pub struct SetRecipe {
    pub target: Entity,
    pub recipe: RecipeListing,
}

#[derive(Component, Debug, Default)]
pub struct SpacecraftGrid {
    parts: usize,
    mass: Mass,
    bounds: (Vec2, Vec2),
    pub center_of_mass: Vec2,
    pub velocity: DVec2,
    pub angular_velocity: f64,
    pub body_frame_acceleration: DVec2,
    pub angular_acceleration: f32,
}

impl SpacecraftGrid {
    pub fn dims(&self) -> Vec2 {
        self.bounds.1 - self.bounds.0
    }

    pub fn apply_body_frame_thrust(&mut self, thrust: Vec2, torque: f32) {
        self.body_frame_acceleration += thrust.as_dvec2() / self.mass.to_kg_f64();
        // TODO change to moment of inertia
        self.angular_acceleration += 0.1 * (torque as f64 / self.mass.to_kg_f64()) as f32;
    }
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct PartInstance(pub starling::prelude::InstantiatedPart);

#[derive(Component, Debug)]
struct PartSprite;

fn draw_grids(mut painter: ShapePainter, crafts: Query<(&GlobalTransform, &SpacecraftGrid)>) {
    for (tf, _) in &crafts {
        painter.reset();
        painter.set_translation(tf.translation().with_z(100.0));
        painter.set_rotation(tf.rotation());
        painter.set_color(TEAL);
        painter.thickness = 4.0;
        painter.hollow = true;
        painter.thickness_type = ThicknessType::Pixels;
        painter.rect(Vec2::ONE * 0.2);
    }
}

fn draw_inventories(
    mut painter: ShapePainter,
    parts: Query<(&GlobalTransform, &PartInstance, &Inventory)>,
) {
    let width = 0.1;
    for (tf, part, inventory) in parts {
        let n = inventory.slot_count();
        let dims = part.prototype().dims_meters();
        let mut small_dims = dims;
        small_dims.y /= n as f32;

        for (i, slot) in inventory.slots().enumerate() {
            let color = if let Some(item) = slot.item() {
                item.color()
            } else {
                BLACK
            };

            let offset =
                dims.y / n as f32 * (i as f32) - (dims.y / 2.0) + (dims.y / n as f32 / 2.0);

            painter.reset();

            painter.set_translation(tf.translation().with_z(10.0) + tf.up() * offset);
            painter.set_rotation(tf.rotation());

            painter.set_color(BLACK.with_alpha(0.6));
            painter.rect(small_dims + Vec2::splat(width * 2.0));

            painter.translate(Vec3::Z);
            painter.set_color(color);
            painter.hollow = false;
            let static_dims = small_dims - Vec2::splat(width * 0.2);
            let mut dyn_dims = static_dims;
            dyn_dims.x *= slot.fill_percentage();
            painter.translate(Vec3::X * (dyn_dims.x / 2.0 - (static_dims.x) / 2.0));
            painter.rect(dyn_dims);
            painter.translate(-Vec3::X * (dyn_dims.x / 2.0 - static_dims.x / 2.0));

            painter.translate(Vec3::Z);
            painter.hollow = true;
            painter.thickness = 2.0;
            painter.thickness_type = ThicknessType::Pixels;
            painter.rect(small_dims - Vec2::splat(width * 0.2));
        }
    }
}

fn draw_selected_grid_guides(
    mut painter: ShapePainter,
    grids: Query<(&GlobalTransform, &SpacecraftGrid)>,
    parts: Query<&ChildOf, With<PartInstance>>,
    cursor: Res<CursorInfo>,
) {
    let id = match cursor.selected {
        Some(c) => c,
        None => return,
    };

    let parent = match parts.get(id) {
        Ok(parent) => parent,
        // might have been deleted. it's fine
        _ => return,
    };

    let (tf, grid) = match grids.get(parent.0) {
        Ok(e) => e,
        // this isn't fine
        Err(e) => {
            error!(?e);
            return;
        }
    };

    painter.reset();
    painter.set_color(RED);
    painter.hollow = true;
    painter.thickness = 4.0;
    painter.thickness_type = ThicknessType::Pixels;
    painter.set_translation(tf.translation().with_z(-50.0));
    painter.set_rotation(tf.rotation());
    painter.rect(grid.dims());

    painter.translate(grid.center_of_mass.extend(100.0));
    painter.hollow = false;
    painter.set_color(GREEN);
    painter.circle(0.1);
    painter.set_color(WHITE);
    painter.circle(0.08);
}

fn draw_selected_part(
    mut painter: ShapePainter,
    parts: Query<(&GlobalTransform, &PartInstance)>,
    sel: Res<CursorInfo>,
    time: Res<Time>,
) {
    let angle = time.elapsed_secs_f64() % (2.0 * std::f64::consts::PI);
    let angle = angle as f32;

    for (color, e, ring) in [
        (TEAL.with_alpha(0.7), sel.hovered, false),
        (ORANGE.with_alpha(0.9), sel.selected, true),
    ] {
        let e = match e {
            Some(e) => e,
            None => continue,
        };

        if let Ok((tf, part)) = parts.get(e) {
            let dims = part.prototype().dims_meters();
            let r = dims.length() / 2.0 + 0.2;
            painter.reset();
            painter.set_translation(tf.translation().with_z(50.0));
            painter.set_rotation(tf.rotation());
            painter.set_color(color);
            painter.thickness = 0.05;
            painter.hollow = true;
            painter.thickness_type = ThicknessType::World;
            painter.rect(dims + Vec2::splat(0.1));
            if ring {
                painter.arc(r, angle, angle + 6.1);
            }
        }
    }
}

fn update_grids(
    mut commands: Commands,
    mut grids: Query<(Entity, &mut SpacecraftGrid, &Children)>,
    parts: Query<(&PartInstance, Option<&Inventory>)>,
) {
    for (e, mut grid, children) in &mut grids {
        grid.mass = Mass::ZERO;
        grid.parts = children.iter().count();
        grid.bounds = (Vec2::ZERO, Vec2::ZERO);
        grid.center_of_mass = Vec2::ZERO;

        if grid.parts == 0 {
            info!("Despawning empty grid {e}");
            commands.entity(e).despawn();
            continue;
        }

        let mut com = DVec2::ZERO;

        for part in children.iter() {
            if let Ok((part, inv)) = parts.get(part) {
                let part_mass = Mass::grams(part.prototype().dry_mass().to_grams());
                let inv_mass = inv.map(|inv| inv.mass()).unwrap_or(Mass::ZERO);
                grid.mass += part_mass + inv_mass;
                com += (part.origin_meters() + part.dims_meters() / 2.0).as_dvec2()
                    * (part_mass + inv_mass).to_kg_f64();
                let origin = part.origin_meters();
                let dims = part.dims_meters();
                grid.bounds.0.x = grid.bounds.0.x.min(origin.x - dims.x);
                grid.bounds.0.y = grid.bounds.0.y.min(origin.y - dims.y);
                grid.bounds.1.x = grid.bounds.1.x.max(origin.x + dims.x);
                grid.bounds.1.y = grid.bounds.1.y.max(origin.y + dims.y);
            } else {
                warn!("Bad grid child: {part}");
            }
        }

        grid.center_of_mass = (com / grid.mass.to_kg_f64()).as_vec2();
    }
}

fn handle_change_recipe(
    mut events: EventReader<SetRecipe>,
    mut machines: Query<(&mut Machine, &mut Inventory)>,
) {
    for event in events.read() {
        info!(?event);
        let (mut machine, mut inv) = match machines.get_mut(event.target) {
            Ok(m) => m,
            Err(e) => {
                error!(?e);
                return;
            }
        };

        machine.set_recipe(event.recipe.clone());

        let recipe = event.recipe.to_recipe();
        *inv = Inventory::from_recipe(&recipe);
    }
}

fn handle_sc_events(
    mut commands: Commands,
    mut events: EventReader<SpacecraftEvent>,
    args: Res<ProgramContext>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut asset_server: ResMut<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    spacecraft: Query<&GlobalTransform, With<SpacecraftGrid>>,
    camera: Query<(&Camera, &GlobalTransform)>,
) -> Result {
    let (camera, transform) = camera.single()?;
    for event in events.read() {
        info!("SpacecraftGrid event: {:?}", event);

        match event {
            SpacecraftEvent::SpawnVehicle { name, pos, angle } => {
                let vehicle_path = args.vehicle_dir().join(format!("{}.vehicle", name));
                let parts = starling::vehicle::load_parts_from_dir(&args.parts_dir())?;
                let vehicle = if let Ok(vehicle) =
                    starling::vehicle::load_vehicle(&vehicle_path, "".to_string(), &parts)
                {
                    vehicle
                } else {
                    commands.send_event(SpawnAnimText::new(format!("Bad vehicle path: {}", name)));
                    continue;
                };

                spawn_spacecraft(
                    &mut commands,
                    *pos,
                    *angle,
                    &vehicle,
                    &mut meshes,
                    &mut asset_server,
                    &args,
                    &mut texture_atlas_layouts,
                );
            }
            SpacecraftEvent::SpawnPart { name, pos, angle } => {
                let parts = starling::vehicle::load_parts_from_dir(&args.parts_dir())?;
                let part = parts.get(name).ok_or("bad part")?;
                let instance = InstantiatedPart::from_prototype(
                    part.clone(),
                    IVec2::ZERO,
                    starling::prelude::Rotation::East,
                );

                let mut grid = spawn_empty_grid(&mut commands, *pos, *angle);
                grid.with_children(|parent| {
                    add_part_to_grid(
                        parent,
                        &instance,
                        &mut meshes,
                        &mut asset_server,
                        &args,
                        &mut texture_atlas_layouts,
                    )
                });
            }
            SpacecraftEvent::Destroy { target } => {
                let tf = spacecraft
                    .get(*target)
                    .map(|v| *v)
                    .unwrap_or(GlobalTransform::default());
                let pos = camera.world_to_viewport(transform, tf.translation());
                commands.entity(*target).despawn();
                commands.send_event(SpawnAnimText {
                    text: "Vehicle deleted".to_string(),
                    color: RED,
                    pos: pos.ok(),
                });
            }
        }
    }

    Ok(())
}

#[derive(Component, Debug, Clone)]
pub struct ConstructionState {
    pub current: usize,
    pub last: usize,
    pub should_build: bool,
}

// randomly increments all ConstructionStates
fn build_parts(
    mut con: Query<(Entity, &mut ConstructionState, &Children)>,
    mut sprites: Query<&mut Sprite, With<PartSprite>>,
    // mut commands: Commands,
) {
    for (_e, mut build, children) in &mut con {
        if rand(0.0, 1.0) < 0.1 && build.should_build {
            if build.current < build.last {
                build.current += 1;
            };
        }

        if build.current == build.last {
            // commands.entity(e).remove::<ConstructionState>();
        }

        for child in children {
            if let Ok(mut sprite) = sprites.get_mut(*child) {
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = build.current;
                }
            }
        }
    }
}

fn spawn_empty_grid<'a>(commands: &'a mut Commands, pos: Vec2, angle: f32) -> EntityCommands<'a> {
    commands.spawn((
        Name::new("Grid"),
        Transform::from_translation(pos.extend(0.0)).with_rotation(Quat::from_rotation_z(angle)),
        SpacecraftGrid::default(),
        Visibility::default(),
        AngularVelocity(0.0),
    ))
}

fn add_part_to_grid<'a>(
    commands: &mut RelatedSpawnerCommands<'a, ChildOf>,
    part: &InstantiatedPart,
    meshes: &mut ResMut<Assets<Mesh>>,
    asset_server: &mut ResMut<AssetServer>,
    args: &Res<ProgramContext>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) {
    let dims = part.prototype().dims_meters();
    let dims_rot = part.dims_meters();
    let origin = part.origin_meters() + dims_rot / 2.0;

    let pixel_dims = part.prototype().dims();

    let (z, _alpha, _t, d) = match part.layer() {
        starling::parts::PartLayer::Internal => (0.0, 1.0, 0.5, 0.0),
        starling::parts::PartLayer::Plumbing => return,
        starling::parts::PartLayer::Structural => (0.02, 0.7, 0.7, 0.05),
        starling::parts::PartLayer::Exterior => (0.04, 0.2, 0.8, 0.1),
    };

    let dims = dims - d;
    let polygon = Rectangle::new(dims.x, dims.y);

    let path = args.part_sprite_path(part.prototype().part_name());
    let texture = asset_server.load(path);

    let name = part.prototype().part_name().to_string();

    let n_sprites = part.prototype().sprites();
    let layout = TextureAtlasLayout::from_grid(pixel_dims, n_sprites as u32, 1, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    let build = ConstructionState {
        current: 0,
        last: n_sprites - 1,
        should_build: true,
    };

    let sprite = Sprite::from_atlas_image(
        texture,
        TextureAtlas {
            layout: texture_atlas_layout,
            index: 0,
        },
    );

    let has_inventory = match part.variant() {
        InstantiatedPartVariant::Thruster(..) => false,
        InstantiatedPartVariant::Tank(..) => true,
        InstantiatedPartVariant::Radar(..) => false,
        InstantiatedPartVariant::Cargo(..) => true,
        InstantiatedPartVariant::Magnetorquer(..) => false,
        InstantiatedPartVariant::Machine(..) => true,
        InstantiatedPartVariant::Generic(..) => false,
    };

    let is_machine = part.as_machine().is_some();
    let is_thruster = part.as_thruster().is_some();
    let is_computer = part.is_computer();
    let is_structural = part.layer() == starling::parts::PartLayer::Structural;

    let n_slots = match part.variant() {
        InstantiatedPartVariant::Cargo(c, _) => c.slots(),
        _ => 1,
    };

    let inv = if is_machine {
        Inventory::zero_slots()
    } else {
        let mut inv = Inventory::zero_slots();
        for _ in 0..n_slots {
            let slot = InvSlot::new(Volume::liters(4000), ItemFilter::Any);
            inv.add_slot(slot.with_item(Item::random()));
        }
        inv
    };

    let mut cmd = commands.spawn((
        Name::new(format!("Part ({})", name)),
        Transform::from_translation(origin.extend(z))
            .with_rotation(Quat::from_rotation_z(part.rotation().to_angle() as f32)),
        PartInstance(part.clone()),
        InheritedVisibility::VISIBLE,
        build,
    ));

    if let Some((model, _)) = part.as_thruster() {
        let mut inv = Inventory::single(Item::H2, Volume::liters(10));
        // inv.fill();

        let thruster = if model.is_rcs {
            Thruster::new(3000.0, true)
        } else {
            Thruster::new(40000.0, false)
        };

        cmd.insert((thruster, inv));
    }

    if is_computer {
        cmd.insert(Computer::default());
    }

    cmd
        // for cursor picking
        .insert_if(Mesh2d(meshes.add(polygon)), || !is_structural)
        .insert_if(Machine::new(RecipeListing::DoNothing), || is_machine)
        .insert_if(inv, || has_inventory)
        .with_child((
            PartSprite,
            sprite,
            Transform::from_scale(Vec3::splat(1.0 / 20.0)),
        ));
}

fn spawn_spacecraft(
    commands: &mut Commands,
    pos: Vec2,
    angle: f32,
    vehicle: &Vehicle,
    meshes: &mut ResMut<Assets<Mesh>>,
    asset_server: &mut ResMut<AssetServer>,
    args: &Res<ProgramContext>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) {
    spawn_empty_grid(commands, pos, angle).with_children(|parent| {
        for (_, part) in vehicle.parts() {
            add_part_to_grid(
                parent,
                part,
                meshes,
                asset_server,
                args,
                texture_atlas_layouts,
            );
        }
    });
}

fn accelerate_spacecraft(
    mut grids: Query<(&mut Transform, &mut SpacecraftGrid)>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs_f64();
    for (mut tf, mut grid) in &mut grids {
        let world_frame_accel = tf
            .rotation
            .mul_vec3(grid.body_frame_acceleration.extend(0.0).as_vec3())
            .xy();
        let da = grid.angular_acceleration as f64 * dt;
        grid.velocity += world_frame_accel.as_dvec2() * dt;
        grid.angular_velocity += da;
        tf.translation += (grid.velocity * dt).as_vec2().extend(0.0);
        tf.rotate_axis(Dir3::Z, (grid.angular_velocity * dt) as f32);
    }
}
