use crate::animated_text::SpawnAnimText;
use crate::inventory::*;
use crate::machine::{Machine, update_machines};
use crate::mass::Mass;
use crate::recipe::*;
use crate::volume::Volume;

use avian2d::prelude::*;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy_ecs::relationship::RelatedSpawnerCommands;
use bevy_vector_shapes::prelude::*;
use game::args::ProgramContext;
use starling::prelude::{InstantiatedPart, InstantiatedPartVariant, Vehicle, rand};
use std::time::Duration;

pub struct SpacecraftPlugin;

impl Plugin for SpacecraftPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    draw_grids,
                    draw_inventories,
                    handle_sc_events,
                    handle_change_recipe,
                    draw_selected_part,
                    draw_selected_grid_guides,
                ),
            )
            .add_systems(FixedUpdate, (build_parts, update_machines))
            .add_systems(
                FixedUpdate,
                update_grids.run_if(on_timer(Duration::from_millis(50))),
            )
            .add_event::<SpacecraftEvent>()
            .add_event::<SetRecipe>()
            .insert_resource(PartCursor::default());
    }
}

#[derive(Resource, Debug, Default)]
pub struct PartCursor {
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
    pub recipe: Option<Recipe>,
}

#[derive(Component, Debug, Default)]
pub struct SpacecraftGrid {
    parts: usize,
    mass: Mass,
    grid_bounds: (IVec2, IVec2),
}

impl SpacecraftGrid {
    pub fn grid_dims(&self) -> IVec2 {
        self.grid_bounds.1 - self.grid_bounds.0
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

            painter.set_color(BLACK);
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
    cursor: Res<PartCursor>,
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
    painter.rect(grid.grid_dims().as_vec2() / 20.0);
}

fn draw_selected_part(
    mut painter: ShapePainter,
    parts: Query<(&GlobalTransform, &PartInstance)>,
    sel: Res<PartCursor>,
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
        grid.grid_bounds = (IVec2::ZERO, IVec2::ZERO);

        if grid.parts == 0 {
            info!("Despawning empty grid {e}");
            commands.entity(e).despawn();
            continue;
        }

        for part in children.iter() {
            if let Ok((part, inv)) = parts.get(part) {
                grid.mass += Mass::grams(part.prototype().dry_mass().to_grams());
                if let Some(inv) = inv {
                    grid.mass += inv.mass();
                }
                let origin = part.origin();
                grid.grid_bounds.0.x = grid.grid_bounds.0.x.min(origin.x);
                grid.grid_bounds.0.y = grid.grid_bounds.0.y.min(origin.y);
                grid.grid_bounds.1.x = grid.grid_bounds.1.x.max(origin.x);
                grid.grid_bounds.1.y = grid.grid_bounds.1.y.max(origin.y);
            } else {
                warn!("Bad grid child: {part}");
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let floor = Rectangle::new(5000.0, 5.0);

    commands.add_observer(
        |mut trigger: Trigger<Pointer<Over>>,
         mut cursor: ResMut<PartCursor>,
         parts: Query<Entity, With<PartInstance>>| {
            let e = if parts.contains(trigger.target()) {
                Some(trigger.target())
            } else {
                return;
            };

            cursor.hovered = e;
            trigger.propagate(false);
        },
    );

    commands.add_observer(
        |mut trigger: Trigger<Pointer<Out>>,
         mut cursor: ResMut<PartCursor>,
         parts: Query<Entity, With<PartInstance>>| {
            let e = if parts.contains(trigger.target()) {
                Some(trigger.target())
            } else {
                return;
            };

            if cursor.hovered == e {
                cursor.hovered = None;
            }
            trigger.propagate(false);
        },
    );

    commands.add_observer(
        |mut trigger: Trigger<Pointer<Click>>,
         mut commands: Commands,
         mut cursor: ResMut<PartCursor>,
         parts: Query<Entity, With<PartInstance>>| {
            let e = if parts.contains(trigger.target()) {
                Some(trigger.target())
            } else {
                return;
            };

            match trigger.button {
                PointerButton::Primary => {
                    cursor.selected = e;
                    trigger.propagate(false);
                }
                PointerButton::Secondary => {
                    e.map(|e| commands.entity(e).despawn());
                    trigger.propagate(false);
                }
                PointerButton::Middle => (),
            };
        },
    );

    commands.spawn((
        Transform::default().rotate_z(30.0f32.to_radians()),
        Collider::from(floor),
        Mesh2d(meshes.add(floor)),
        MeshMaterial2d(materials.add(Color::from(GRAY.with_alpha(0.4)))),
    ));
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

        if let Some(recipe) = &event.recipe {
            *inv = Inventory::from_recipe(recipe);
        } else {
            *inv = Inventory::zero_slots();
        }
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

    commands
        .spawn((
            Name::new(format!("Part ({})", name)),
            Transform::from_translation(origin.extend(z))
                .with_scale(Vec3::splat(1.0))
                .with_rotation(Quat::from_rotation_z(part.rotation().to_angle() as f32)),
            PartInstance(part.clone()),
            InheritedVisibility::VISIBLE,
            build,
        ))
        .insert_if(Mesh2d(meshes.add(polygon)), || has_inventory)
        .insert_if(Machine::new(None), || is_machine)
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
