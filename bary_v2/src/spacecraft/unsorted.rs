use crate::*;
use bary_core::prelude::*;
use bary_v1::args::ProgramContext;
use bevy::color::palettes::css::*;
use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use bevy_ecs::relationship::RelatedSpawnerCommands;
use bevy_vector_shapes::prelude::*;

pub struct SpacecraftPlugin;

impl Plugin for SpacecraftPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                draw_grids,
                draw_inventories,
                draw_blueprints,
                draw_selected_part_system,
                draw_spacecraft_spatial_lookups,
                draw_docking_info,
            ),
        );

        app.add_systems(
            EguiPrimaryContextPass,
            (
                tick_control_egui,
                docking_program_egui.pipe(swallow_optional),
                hose_info_window_egui_system.pipe(swallow_result),
            ),
        );

        app.add_systems(
            Update,
            (
                draw_blueprint_of_docking_program.pipe(swallow_optional),
                draw_position_command_widget,
                update_position_command_widget_system,
                spawn_hose_on_keypress_system.pipe(swallow_optional),
                draw_hoses_system,
                update_selected_spacecraft_system,
                update_selected_hose_system,
                draw_hose_selection_area_system,
                process_position_commands_system.pipe(swallow_optional),
            ),
        );

        app.add_systems(FixedUpdate, world_tick_driver_system);

        app.add_systems(
            SimTick,
            (
                handle_sc_events,
                check_adjacent_docking_ports,
                build_parts,
                update_machines,
                accelerate_spacecraft,
                despawn_empty_grids,
                update_grids,
                update_spacecraft_grid_map,
                update_hose_physics_system,
                do_hose_inventory_transfer_system,
                process_docking_triggers,
            )
                .chain(),
        );

        app.add_event::<SpacecraftEvent>();
        app.add_event::<PositionHoldCommand>();
        app.add_event::<DockingTrigger>();

        app.insert_resource(SelectedSpacecraft::default());
        app.insert_resource(SelectedHose::default());
        app.insert_resource(GridSpatialLookup::default());
        app.insert_resource(TickSchedule::PerFrame(10));
        app.insert_resource(CursorPositionCommandWidget::default());
        app.insert_resource(DockingProgram::default());
        app.insert_resource(TickStatistics::default());
    }
}

#[derive(Event, Debug)]
pub enum SpacecraftEvent {
    SpawnVehicle {
        ship_name: String,
        blueprint_name: String,
        pos: Vec2,
        angle: f32,
    },
    SpawnPart {
        name: String,
        pos: Vec2,
        angle: f32,
    },
    Destroy {
        target: Entity,
    },
}

#[derive(Component, Debug, Default)]
pub struct SpacecraftGrid {
    parts: usize,
    inventory_mass: Mass,
    parts_mass: Mass,
    fuel_mass: Mass,
    moment_of_inertia: f64,
    bounds: (Vec2, Vec2),
    pub center_of_mass: Vec2,
    pub velocity: DVec2,
    pub angular_velocity: f64,
    pub body_frame_acceleration: DVec2,
    pub angular_acceleration: f32,
    pub is_dirty: bool,
}

impl SpacecraftGrid {
    pub fn dims(&self) -> Vec2 {
        self.bounds.1 - self.bounds.0
    }

    pub fn total_mass(&self) -> Mass {
        self.parts_mass + self.inventory_mass + self.fuel_mass
    }

    pub fn apply_body_frame_thrust(&mut self, thrust: Vec2, torque: f32) {
        self.body_frame_acceleration += thrust.as_dvec2() / self.total_mass().to_kg_f64();
        // TODO change to moment of inertia
        self.angular_acceleration += (torque as f64 / self.moment_of_inertia) as f32;
    }
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct PartInstance(pub InstantiatedPart);

#[derive(Component, Debug)]
struct PartSprite;

fn draw_grids(
    mut painter: ShapePainter,
    crafts: Query<(&GlobalTransform, &SpacecraftGrid)>,
    settings: Res<Settings>,
) {
    if !settings.draw_spacecraft_grids {
        return;
    }

    const Z_SPACECRAFT_GRID_MARKERS: f32 = 100.0;

    for (tf, grid) in &crafts {
        painter.reset();
        painter.set_translation(tf.translation().with_z(Z_SPACECRAFT_GRID_MARKERS));
        painter.set_rotation(tf.rotation());
        painter.set_color(TEAL);
        painter.thickness = 6.0;
        painter.hollow = true;
        painter.thickness_type = ThicknessType::Pixels;
        painter.rect(Vec2::ONE * 0.4);

        painter.translate(grid.center_of_mass.extend(Z_SPACECRAFT_GRID_MARKERS));
        painter.set_color(GREEN);
        painter.rect(Vec2::ONE * 0.4);
    }
}

fn draw_blueprints(
    mut gizmos: Gizmos,
    bps: Query<(&Blueprint, &Transform)>,
    settings: Res<Settings>,
    parts: Res<PartsResource>,
) {
    if !settings.draw_blueprints {
        return;
    }

    for (bp, tf) in bps {
        draw_blueprint(&mut gizmos, bp, *tf, &parts);
    }
}

fn draw_inventories(
    mut painter: ShapePainter,
    parts: Query<(&GlobalTransform, &PartInstance, &Inventory)>,
    settings: Res<Settings>,
) {
    if !settings.draw_inventories {
        return;
    }

    const Z_DEBUG_INVENTORY_LAYER: f32 = 0.05;

    for (tf, part, inventory) in parts {
        for slot in inventory.slots() {
            let (min, max) = slot.bounds();
            let c = (max + min).to_meters() / 2.0;
            let d = (max - min).to_meters();
            let half_width = part.placement.part_aligned_dims().to_meters() / 2.0;

            painter.reset();

            painter.set_translation(tf.translation().with_z(Z_DEBUG_INVENTORY_LAYER));
            painter.set_rotation(tf.rotation());
            painter.translate((c - half_width).extend(0.0));

            // backdrop
            painter.set_color(GRAY_900);
            painter.rect(d + Vec2::splat(0.3));
            painter.translate(Vec3::Z * 0.01);

            // slot background
            painter.set_color(GRAY_800);
            painter.rect(d + Vec2::splat(0.05));
            painter.translate(Vec3::Z * 0.01);

            let slot_size = d - Vec2::splat(0.08);

            if let Some(item) = slot.item() {
                let color = item.color();
                painter.set_color(color);
                let p = slot.fill_percentage();
                painter.rect(slot_size * p);
            };

            painter.hollow = true;
            painter.thickness = 0.01;
            painter.thickness_type = ThicknessType::World;
            painter.set_color(GRAY_500);
            painter.rect(slot_size);
        }
    }
}

fn despawn_empty_grids(
    mut commands: Commands,
    grids: Query<Entity, (With<SpacecraftGrid>, Without<Children>)>,
) {
    for e in grids {
        info!("Despawning empty grid {e}");
        commands.entity(e).despawn();
    }
}

#[derive(Component)]
pub struct FuelInventory;

fn update_grids(
    mut commands: Commands,
    mut grids: Query<(Entity, &mut SpacecraftGrid, &Children)>,
    parts: Query<(&PartInstance, Option<&Inventory>, Option<&FuelInventory>)>,
    part_db: Res<PartsResource>,
) {
    // TODO: we should only run this when a given grid has changed.
    // certain events should trigger a change event:
    //  - adding/removing a part
    //  - starting/ending a recipe
    //  - thrusting
    //  - damage?
    //  - etc

    for (e, mut grid, children) in &mut grids {
        if !grid.is_dirty {
            continue;
        }

        info!("Updating grid {}", e);

        grid.parts_mass = Mass::ZERO;
        grid.inventory_mass = Mass::ZERO;
        grid.fuel_mass = Mass::ZERO;
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
            if let Ok((part, inv, fuel)) = parts.get(part) {
                let Some(proto) = part_db.get(&part.name) else {
                    continue;
                };
                let part_mass = Mass::grams(proto.part_mass().to_grams());
                let inv_mass = inv.map(|inv| inv.mass()).unwrap_or(Mass::ZERO);
                if fuel.is_some() {
                    grid.fuel_mass += inv_mass;
                } else {
                    grid.inventory_mass += inv_mass;
                }
                grid.parts_mass += part_mass;
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

        grid.moment_of_inertia = grid.total_mass().to_kg_f64() * 10.0;
        grid.center_of_mass = (com / grid.total_mass().to_kg_f64()).as_vec2();

        grid.is_dirty = false;
    }
}

fn handle_sc_events(
    mut commands: Commands,
    mut events: EventReader<SpacecraftEvent>,
    args: Res<ProgramContext>,
    mut asset_server: ResMut<AssetServer>,
    spacecraft: Query<&GlobalTransform, With<SpacecraftGrid>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    parts: Res<PartsResource>,
) -> Result {
    let (camera, transform) = camera.single()?;
    for event in events.read() {
        info!("SpacecraftGrid event: {:?}", event);

        match event {
            SpacecraftEvent::SpawnVehicle {
                ship_name,
                blueprint_name,
                pos,
                angle,
            } => {
                let vehicle_path = args
                    .vehicle_dir()
                    .join(format!("{}.vehicle", blueprint_name));
                let vehicle = if let Ok(vehicle) = load_vehicle(&vehicle_path, &parts) {
                    vehicle
                } else {
                    commands.send_event(SpawnAnimText::new(format!(
                        "Bad vehicle path: {}",
                        blueprint_name
                    )));
                    panic!();
                };

                spawn_spacecraft(
                    &mut commands,
                    *pos,
                    *angle,
                    ship_name.clone(),
                    &vehicle,
                    &mut asset_server,
                    &args,
                    &parts,
                );
            }
            SpacecraftEvent::SpawnPart { name, pos, angle } => {
                let parts = load_parts_from_dir(&args.parts_dir())?;
                let part = parts.get(name).ok_or("bad part")?;
                let instance = InstantiatedPart::from_prototype(
                    part.clone(),
                    PartCoord::new(IVec2::ZERO),
                    bary_core::prelude::Rotation::East,
                );

                let mut grid = spawn_empty_grid(&mut commands, *pos, *angle, "Grid".to_string());
                grid.with_children(|parent| {
                    add_part_to_grid(parent, &instance, &mut asset_server, &args, &parts)
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

fn spawn_empty_grid<'a>(
    commands: &'a mut Commands,
    pos: Vec2,
    angle: f32,
    name: String,
) -> EntityCommands<'a> {
    commands.spawn((
        Name::new(name),
        Transform::from_translation(pos.extend(0.0)).with_rotation(Quat::from_rotation_z(angle)),
        SpacecraftGrid {
            // velocity: randvec(2.0, 4.0).as_dvec2() / 3.0,
            // angular_velocity: 0.3,
            is_dirty: true,
            ..default()
        },
        Visibility::default(),
        Blueprint::new(),
    ))
}

fn add_part_to_grid<'a>(
    commands: &mut RelatedSpawnerCommands<'a, ChildOf>,
    part: &InstantiatedPart,
    asset_server: &mut ResMut<AssetServer>,
    args: &Res<ProgramContext>,
    parts: &PartDatabase,
) {
    let dims = part.placement.part_aligned_dims().to_meters();
    let dims_rot = part.dims_meters();
    let origin = part.origin_meters() + dims_rot / 2.0;

    let (z, _, _, _) = match part.layer() {
        PartLayer::Internal => (0.0, 1.0, 0.5, 0.0),
        PartLayer::Structural => (0.02, 0.7, 0.7, 0.05),
        PartLayer::Exterior => (0.04, 0.2, 0.8, 0.1),
        _ => return,
    };

    let path = args.part_sprite_path(&part.name);
    let texture = asset_server.load(path);

    let mut sprite = Sprite::from_image(texture);

    let proto = parts.get(&part.name).unwrap();

    sprite.custom_size = Some(dims);

    let mut cmd = commands.spawn((
        Transform::from_translation(origin.extend(z))
            .with_rotation(Quat::from_rotation_z(part.rotation().to_angle() as f32)),
        PartInstance(part.clone()),
        InheritedVisibility::VISIBLE,
    ));

    // INVENTORY COMPONENT ==================================================

    if let Some(data) = InstantiatedPart::inventory_data(proto) {
        let mut inv = Inventory::zero_slots();
        for data in &data.slots {
            let bounds = (data.min, data.max);
            let mut slot = InvSlot::new(
                Volume::liters_f32(data.volume_liters),
                data.filter.clone(),
                bounds,
            );

            slot.set_name(data.name.clone());

            inv.add_slot(slot);
        }
        cmd.insert(inv);
    }

    // MACHINE COMPONENT ==================================================

    if let Some(data) = InstantiatedPart::machine_data(proto) {
        // TODO use the data
        let machine = Machine::new(RecipeListing::DoNothing);
        cmd.insert(machine);
    }

    // THRUSTER COMPONENT ==================================================

    if let Some(model) = InstantiatedPart::thruster_data(proto) {
        let thruster = if model.is_rcs {
            Thruster::new(3000.0, true)
        } else {
            Thruster::new(40000.0, false)
        };
        cmd.insert((thruster, FuelInventory));
    }

    // COMPUTER COMPONENT ==================================================

    if let Some(cpu) = InstantiatedPart::computer_data(proto) {
        let mut cpu = Computer::default();
        cpu.mode = ComputerMode::Manual;
        cpu.attitude = rand(0.0, 2.0);
        cpu.on = false;
        cmd.insert(cpu);
    }

    // EXCAVATOR COMPONENT ==================================================

    if let Some(data) = InstantiatedPart::excavator_data(proto) {
        cmd.insert(Excavator::new(data.radius));
    }

    // DOCKING PORT COMPONENT ===============================================

    if let Some(data) = InstantiatedPart::docking_port_data(proto) {
        let docking = DockingPort::new(data.distance);
        cmd.insert(docking);
    }

    // SPRITE CHILD ENTITY ===============================================

    cmd.with_child((PartSprite, sprite));
}

fn spawn_spacecraft(
    commands: &mut Commands,
    pos: Vec2,
    angle: f32,
    name: String,
    vehicle: &Blueprint,
    asset_server: &mut ResMut<AssetServer>,
    args: &Res<ProgramContext>,
    parts: &PartDatabase,
) {
    spawn_empty_grid(commands, pos, angle, name)
        .with_children(|parent| {
            for (_, part) in vehicle.parts() {
                add_part_to_grid(parent, part, asset_server, args, parts);
            }
        })
        .insert(vehicle.clone());
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
        let ds = (grid.velocity * dt).as_vec2();
        tf.translation += ds.extend(0.0);
        tf.rotate_axis(Dir3::Z, (grid.angular_velocity * dt) as f32);
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct GridSpatialLookup(HashMap<IVec2, Vec<Entity>>);

impl GridSpatialLookup {
    pub fn lup(&self, pos: Vec2) -> Option<&Vec<Entity>> {
        let g = to_grid(pos);
        self.0.get(&g)
    }

    pub fn add(&mut self, g: IVec2, e: Entity) {
        if let Some(v) = self.get_mut(&g) {
            v.push(e);
        } else {
            self.insert(g, vec![e]);
        }
    }
}

fn grids_in_radius(p: Vec2, r: f32) -> (IVec2, IVec2) {
    let lower = p - Vec2::splat(r);
    let upper = p + Vec2::splat(r);
    let lg = to_grid(lower);
    let ug = to_grid(upper);
    (lg, ug)
}

fn update_spacecraft_grid_map(
    grids: Query<(Entity, &GlobalTransform, &SpacecraftGrid)>,
    mut map: ResMut<GridSpatialLookup>,
) {
    map.clear();
    for (e, transform, grid) in grids {
        let p = transform.translation().xy();
        let r = grid.dims().length();
        let (lower, upper) = grids_in_radius(p, r);

        for x in lower.x..=upper.x {
            for y in lower.y..=upper.y {
                map.add(IVec2::new(x, y), e);
            }
        }
    }
}

fn draw_spacecraft_spatial_lookups(
    mut painter: ShapePainter,
    map: Res<GridSpatialLookup>,
    transforms: Query<&GlobalTransform>,
    settings: Res<Settings>,
) {
    if !settings.draw_spatial_lut {
        return;
    }

    const Z_SPATIAL_LUT_DEBUG: f32 = 20.0;

    for (g, entities) in map.iter() {
        painter.reset();
        painter.set_color(ORANGE.with_alpha(0.1));
        painter.hollow = true;
        painter.thickness_type = ThicknessType::Pixels;
        painter.thickness = 12.0;
        let (lower, upper) = chunk_bounds(*g);
        let center = (upper + lower) / 2.0;
        painter.set_translation(center.extend(Z_SPATIAL_LUT_DEBUG));
        painter.rect(upper - lower);

        for e in entities {
            let tf = ok_or_continue!(transforms.get(*e));
            painter.reset();
            painter.set_translation(tf.translation().with_z(Z_SPATIAL_LUT_DEBUG));
            painter.set_color(RED.with_alpha(0.3));
            painter.circle(2.0);
        }
    }
}
