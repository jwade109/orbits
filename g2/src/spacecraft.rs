use crate::animated_text::SpawnAnimText;
use avian2d::prelude::*;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy_ecs::relationship::RelatedSpawnerCommands;
use bevy_vector_shapes::prelude::*;
use game::args::ProgramContext;
use starling::prelude::*;
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
                    handle_events,
                    on_change_grid_info,
                    draw_selected_part,
                ),
            )
            .add_systems(FixedUpdate, (build_parts, update_machines))
            .add_systems(
                FixedUpdate,
                update_grids.run_if(on_timer(Duration::from_secs(1))),
            )
            .add_event::<SpacecraftEvent>()
            .insert_resource(SelectedPart(None));
    }
}

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct SelectedPart(pub Option<Entity>);

#[derive(Event, Debug)]
pub enum SpacecraftEvent {
    SpawnVehicle { name: String, pos: Vec2, angle: f32 },
    SpawnPart { name: String, pos: Vec2, angle: f32 },
    Destroy { target: Entity },
}

#[derive(Component, Debug, Default, Reflect)]
pub struct SpacecraftGrid {
    parts: usize,
    mass: f32,
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct PartInstance(pub starling::prelude::PartPrototype);

#[derive(Component, Debug)]
struct PartSprite;

fn draw_grids(mut painter: ShapePainter, crafts: Query<(&GlobalTransform, &SpacecraftGrid)>) {
    for (tf, grid) in &crafts {
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

fn draw_selected_part(
    mut painter: ShapePainter,
    parts: Query<(&GlobalTransform, &PartInstance)>,
    sel: Res<SelectedPart>,
    time: Res<Time>,
) {
    let angle = time.elapsed_secs_f64() % (2.0 * std::f64::consts::PI);
    let angle = angle as f32;

    let e = match sel.0 {
        Some(e) => e,
        None => return,
    };

    if let Ok((tf, part)) = parts.get(e) {
        let r = part.dims_meters().length() / 2.0 + 0.2;
        painter.reset();
        painter.set_translation(tf.translation().with_z(10.0));
        painter.set_rotation(tf.rotation());
        painter.set_color(ORANGE);
        painter.thickness = 0.05;
        painter.hollow = true;
        painter.thickness_type = ThicknessType::World;
        painter.rect(part.dims_meters() + Vec2::splat(0.1));
        painter.arc(r, angle, angle + 6.1);
    }
}

fn on_change_grid_info(grids: Query<(Entity, &SpacecraftGrid), Changed<SpacecraftGrid>>) {
    for (e, grid) in &grids {
        info!("Changed grid: {e}, {grid:?}");
    }
}

fn update_machines(mut machines: Query<(&mut Machine, &mut Inventory)>) {
    for (mut m, mut inv) in &mut machines {
        m.step_process(&mut inv);
    }
}

fn update_grids(
    mut commands: Commands,
    mut grids: Query<(Entity, &mut SpacecraftGrid, &Children)>,
    parts: Query<&mut PartInstance>,
) {
    for (e, mut grid, children) in &mut grids {
        let mut mass = 0.0;
        let n_parts = children.iter().count();

        if n_parts == 0 {
            info!("Despawning empty grid {e}");
            commands.entity(e).despawn();
            continue;
        }

        for part in children.iter() {
            if let Ok(part) = parts.get(part) {
                mass += part.dry_mass().to_kg_f64() as f32;
            } else {
                warn!("Bad grid child: {part}");
            }
        }

        // don't trigger change detection unless needed
        if grid.mass != mass {
            grid.mass = mass;
        }

        if grid.parts != n_parts {
            grid.parts = n_parts;
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
        |mut trigger: Trigger<Pointer<Drag>>,
         parts: Query<(&Name, &ChildOf)>,
         mut sc: Query<
            (&Name, &mut LinearVelocity, &mut AngularVelocity),
            With<SpacecraftGrid>,
        >| {
            if let Ok((_, child_of)) = parts.get(trigger.target()) {
                if let Ok((_, mut vel, mut ang)) = sc.get_mut(child_of.0) {
                    let d = trigger.event().delta / 10.0;
                    vel.x += d.x;
                    vel.y += -d.y;
                    ang.0 *= 0.95;
                }
            }
            trigger.propagate(false);
        },
    );

    commands.add_observer(
        |mut trigger: Trigger<Pointer<Click>>,
         mut commands: Commands,
         mut current: ResMut<SelectedPart>,
         parts: Query<Entity, With<PartInstance>>| {
            let e = if parts.contains(trigger.target()) {
                Some(trigger.target())
            } else {
                return;
            };

            match trigger.button {
                PointerButton::Primary => {
                    current.0 = e;
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

fn handle_events(
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

#[derive(Component, Debug, Clone, Deref, DerefMut)]
pub struct Inventory(pub starling::prelude::Inventory);

#[derive(Component, Debug, Clone)]
pub struct Machine {
    pub enabled: bool,
    pub steps: u32,
    pub required_steps: u32,
    pub recipe: Recipe,
    pub products_finished: u64,
    pub status: MachineStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineStatus {
    Off,
    Running,
    NoRoom,
    Starved,
}

impl Machine {
    pub fn new(recipe: Recipe) -> Self {
        Self {
            enabled: false,
            recipe,
            steps: 0,
            required_steps: randint(20, 32) as u32,
            products_finished: 0,
            status: MachineStatus::Off,
        }
    }

    pub fn is_running(&self) -> bool {
        self.status == MachineStatus::Running
    }

    pub fn progress(&self) -> f32 {
        self.steps as f32 / self.required_steps as f32
    }

    pub fn set_recipe(&mut self, recipe: Recipe) {
        self.recipe = recipe;
        self.steps = 0;
    }

    fn take_inputs_if_possible(&self, inv: &mut Inventory) -> bool {
        let has_all = self
            .recipe
            .inputs()
            .all(|(item, count)| inv.count(item) >= count);

        if has_all {
            for (item, count) in self.recipe.inputs() {
                inv.take(item, count);
            }
        }

        has_all
    }

    fn put_inputs_if_possible(&self, inv: &mut Inventory) -> bool {
        let can_put_all = self
            .recipe
            .outputs()
            .all(|(item, count)| inv.can_store(item, count));

        if can_put_all {
            for (item, count) in self.recipe.outputs() {
                inv.add(item, count);
            }
        }

        can_put_all
    }

    pub fn step_process(&mut self, inv: &mut Inventory) {
        if !self.enabled {
            self.status = MachineStatus::Off;
            return;
        }

        if self.steps == 0 {
            if self.take_inputs_if_possible(inv) {
                self.steps += 1;
                self.status = MachineStatus::Running;
                return;
            } else {
                self.status = MachineStatus::Starved;
                return;
            }
        }

        if self.steps > 0 && self.steps < self.required_steps {
            self.status = MachineStatus::Running;
            self.steps += 1;
        } else if self.steps >= self.required_steps {
            if self.put_inputs_if_possible(inv) {
                self.steps = 0;
                self.products_finished += 1;
                self.status = MachineStatus::Running;
            } else {
                self.status = MachineStatus::NoRoom;
            }
        } else {
            self.status = MachineStatus::Off;
        }
    }
}

impl std::fmt::Display for Machine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "enabled={}, recipe={}", self.enabled, self.recipe)
    }
}

impl Inventory {
    fn new() -> Self {
        Self(starling::prelude::Inventory::random())
    }
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

    use starling::prelude::Item;

    let recipe = Recipe::consumes(Item::Ice, 40).and_produces(Item::Water, 600);

    commands
        .spawn((
            Name::new(format!("Part ({})", name)),
            Transform::from_translation(origin.extend(z))
                .with_scale(Vec3::splat(1.0))
                .with_rotation(Quat::from_rotation_z(part.rotation().to_angle() as f32)),
            Mesh2d(meshes.add(polygon)),
            PartInstance(part.prototype()),
            build,
        ))
        .insert_if(Inventory::new(), || has_inventory)
        .insert_if(Machine::new(recipe), || is_machine)
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
