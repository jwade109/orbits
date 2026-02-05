use crate::sounds::SoundSource;
use crate::system_sets::*;
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
                draw_debug_inventory_links,
                draw_grid_placement_effects,
            )
                .in_set(DrawSet),
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
                update_position_command_widget_system,
                update_slingshot_widget_system,
                update_selected_spacecraft_system,
                update_selected_hose_system,
                update_grid_placement_effects,
            ),
        );

        app.add_systems(
            Update,
            (
                draw_blueprint_of_docking_program.pipe(swallow_optional),
                draw_position_command_widget,
                draw_slingshot_widget,
                draw_hoses_system,
                draw_hose_selection_area_system,
                despawn_grid_placement_effects,
            )
                .in_set(DrawSet),
        );

        app.add_observer(on_add_hose.pipe(swallow_optional));
        app.add_observer(on_add_pipe.pipe(swallow_result));

        app.add_systems(FixedUpdate, world_tick_driver_system);

        app.add_systems(
            SimTick,
            (
                check_adjacent_docking_ports,
                update_machines,
                accelerate_spacecraft,
                despawn_empty_grids,
                update_grids,
                update_spacecraft_grid_map,
                update_hose_physics_system,
                process_inventory_links_system,
                update_computers,
                do_maneuvers,
            )
                .in_set(SimulationSet::Misc),
        );

        app.add_observer(handle_sc_events);
        app.add_observer(on_attach_part_to_grid);
        app.add_observer(process_docking_triggers);
        app.add_observer(on_position_commands_system.pipe(swallow_optional));
        app.add_observer(emit_text_alert_on_position_hold);
        app.add_observer(on_delta_v_observer.pipe(swallow_result));

        app.insert_resource(SelectedSpacecraft::default());
        app.insert_resource(SelectedHose::default());
        app.insert_resource(GridSpatialLookup::default());
        app.insert_resource(TickSchedule::PerFrame(10));
        app.insert_resource(CursorPositionCommandWidget::default());
        app.insert_resource(SlingshotWidget::default());
        app.insert_resource(DockingProgram::default());
        app.insert_resource(TickStatistics::default());
    }
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
        let iso = transform_to_isometry(*tf);
        draw_blueprint(&mut gizmos, bp, iso, &parts);
    }
}

fn draw_inventories(
    mut painter: ShapePainter,
    inventory: InventoryApi,
    transforms: TransformHelper,
    parts: Query<(Entity, &PartInstance, &PartContainers)>,
    settings: Res<Settings>,
) {
    if !settings.draw_inventories {
        return;
    }

    const Z_DEBUG_INVENTORY_LAYER: f32 = 0.05;

    for (e, part, containers) in parts {
        let tf = ok_or_continue!(transforms.compute_global_transform(e));

        for entity in containers.iter() {
            let (loc, slot) = ok_or_continue!(inventory.get_container(entity));
            let (min, max) = (loc.origin, loc.origin + loc.dims);
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

            if slot.is_fluid() {
                painter.corner_radii = Vec4::splat(2.0);
            }

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
    grids: Query<(Entity, &Name), (With<SpacecraftGrid>, Without<GridParts>)>,
) {
    for (e, name) in grids {
        info!("Despawning empty grid {e}: {}", name);
        commands.entity(e).despawn();
    }
}

#[derive(Component)]
pub struct ThrusterInventory(pub bool);

fn update_grids(
    mut commands: Commands,
    mut grids: Query<(Entity, &mut SpacecraftGrid, &GridParts)>,
    parts: Query<&PartInstance>,
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
            if let Ok(part) = parts.get(part) {
                let Some(proto) = part_db.get(&part.name) else {
                    continue;
                };
                let part_mass = Mass::grams(proto.part_mass().to_grams());
                grid.parts_mass += part_mass;
                let inv_mass = Mass::ZERO;
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
    event: On<SpacecraftEvent>,
    mut commands: Commands,
    args: Res<ProgramContext>,
    mut asset_server: ResMut<AssetServer>,
    spacecraft: Query<&GlobalTransform, With<SpacecraftGrid>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    parts: Res<PartsResource>,
) -> Result {
    let (camera, transform) = camera.single()?;
    info!("SpacecraftGrid event: {:?}", event);

    match event.event() {
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
                commands.write_message(SpawnAnimText::new(format!(
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
            let instance = PartInstance::from_prototype(
                part.clone(),
                PartCoord::new(IVec2::ZERO),
                bary_core::prelude::Rotation::East,
            );

            let grid_id = spawn_empty_grid(&mut commands, *pos, *angle, "Grid".to_string()).id();
            add_part_to_grid(
                &mut commands,
                grid_id,
                &instance,
                &mut asset_server,
                &args,
                &parts,
            );
        }
        SpacecraftEvent::Destroy { target } => {
            let tf = spacecraft
                .get(*target)
                .map(|v| *v)
                .unwrap_or(GlobalTransform::default());
            let pos = camera.world_to_viewport(transform, tf.translation());
            commands.entity(*target).despawn();
            commands.write_message(SpawnAnimText {
                text: "Vehicle deleted".to_string(),
                color: RED,
                pos: pos.ok(),
                target: None,
            });
        }
    }

    Ok(())
}

fn on_attach_part_to_grid(event: On<AttachPart>) {
    info!("Attaching part: {:?}", event);
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
            is_dirty: true,
            ..default()
        },
        Visibility::default(),
        Blueprint::new(),
    ))
}

fn add_part_to_grid(
    commands: &mut Commands,
    grid_id: Entity,
    part: &PartInstance,
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

    let part_id = commands
        .spawn((
            Transform::from_translation(origin.extend(z))
                .with_rotation(Quat::from_rotation_z(part.rotation().to_angle() as f32)),
            part.clone(),
            InheritedVisibility::VISIBLE,
            ChildOf(grid_id),
            PartInGrid(grid_id),
        ))
        .id();

    // INVENTORY COMPONENT ==================================================

    let is_thruster = PartInstance::thruster_data(proto).is_some();

    if let Some(data) = PartInstance::inventory_data(proto) {
        for data in &data.slots {
            let mut slot = InvSlot::new(
                Volume::liters_f32(data.volume_liters),
                data.filter.clone(),
                data.is_fluid.unwrap_or(false),
            );

            slot.set_name(data.name.clone());

            let loc = ContainerLocation {
                origin: data.min.into(),
                dims: (data.max - data.min).into(),
            };

            commands.spawn((
                slot,
                loc,
                ContainerInPart(part_id),
                ThrusterInventory(is_thruster),
            ));
        }
    }

    // MACHINE COMPONENT ==================================================

    if let Some(data) = PartInstance::machine_data(proto) {
        // TODO use the data
        let machine = Machine::from_data(data.clone());
        commands.entity(part_id).insert(machine);
    }

    // THRUSTER COMPONENT ==================================================

    if let Some(model) = PartInstance::thruster_data(proto) {
        let thruster = if model.is_rcs {
            Thruster::new(3000.0, true)
        } else {
            Thruster::new(40000.0, false)
        };

        commands
            .entity(part_id)
            .insert((thruster, SoundSource("thrust-noise.ogg".into())));
    }

    // COMPUTER COMPONENT ==================================================

    if let Some(data) = PartInstance::computer_data(proto) {
        let mut cpu = Computer::default();
        cpu.mode = ComputerMode::Manual;
        cpu.ticks_per_cycle = data.ticks_per_cycle;
        cpu.attitude = rand(0.0, 2.0);
        cpu.on = false;
        commands.entity(part_id).insert(cpu);
    }

    // EXCAVATOR COMPONENT ==================================================

    if let Some(data) = PartInstance::excavator_data(proto) {
        commands.entity(part_id).insert(Excavator::new(data.radius));
    }

    // DOCKING PORT COMPONENT ===============================================

    if let Some(data) = PartInstance::docking_port_data(proto) {
        let docking = DockingPort::new(data.distance);
        commands.entity(part_id).insert(docking);
    }

    // SPRITE CHILD ENTITY ===============================================

    commands.spawn((PartSprite, sprite, ChildOf(part_id)));
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
    let grid_id = spawn_empty_grid(commands, pos, angle, name).id();
    for (_, part) in vehicle.parts() {
        add_part_to_grid(commands, grid_id, part, asset_server, args, parts);
    }
    commands.entity(grid_id).insert(vehicle.clone());
}

fn accelerate_spacecraft(mut grids: Query<(&mut Transform, &mut SpacecraftGrid)>) {
    let dt = 1.0 / 60.0;
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
