use crate::args::ProgramContext;
use crate::new::animated_text::SpawnAnimText;
use avian2d::prelude::*;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_light_2d::prelude::*;
use bevy_vector_shapes::prelude::*;
use starling::prelude::{rand, Vehicle};

pub struct SpacecraftPlugin;

impl Plugin for SpacecraftPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .add_systems(Startup, setup)
            .add_systems(Update, (draw_spacecraft, handle_events))
            .add_systems(FixedUpdate, blink_lights)
            .add_event::<SpacecraftEvent>();
    }
}

fn blink_lights(mut query: Query<(&mut PointLight2d, &mut ShipLight)>, time: Res<Time<Fixed>>) {
    let dt = time.delta_secs();
    for (mut light, mut dur) in &mut query {
        dur.0 -= dt;
        if dur.0 < 0.0 {
            if light.intensity > 0.0 {
                light.intensity = 0.0;
            } else {
                light.intensity = 20.0;
            }
            dur.0 = 1.0;
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SpacecraftView {
    Real,
    Type,
    Damage,
}

#[derive(Event, Debug)]
pub enum SpacecraftEvent {
    Spawn { name: String, pos: Vec2 },
    Destroy { target: Entity },
    ToggleView(SpacecraftView),
}

#[derive(Component, Debug)]
struct Spacecraft;

#[derive(Component, Debug)]
struct PartInstance(starling::prelude::PartPrototype);

#[derive(Component, Debug)]
struct PartSprite;

#[derive(Component, Debug)]
struct Selected;

#[derive(Component, Debug)]
struct Hovered;

#[derive(Component, Debug)]
struct ShipLight(f32);

fn draw_spacecraft(
    mut painter: ShapePainter,
    transforms: Query<&GlobalTransform>,
    crafts: Query<(&GlobalTransform, Option<&Selected>), With<Spacecraft>>,
) {
    for (craft, selected) in &crafts {
        painter.reset();
        painter.set_translation(craft.translation());
        let color = if selected.is_some() {
            ORANGE
        } else {
            WHITE.with_alpha(0.5)
        };
        painter.set_color(color);
        painter.thickness = 4.0;
        painter.hollow = true;
        painter.thickness_type = ThicknessType::Pixels;
        painter.circle(10.0);
    }

    for tf in &transforms {
        painter.reset();
        painter.set_translation(tf.translation());
        painter.set_color(WHITE.with_alpha(0.2));
        painter.thickness = 2.0;
        painter.hollow = true;
        painter.thickness_type = ThicknessType::Pixels;
        painter.circle(0.1);
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
         mut sc: Query<(&Name, &mut LinearVelocity, &mut AngularVelocity), With<Spacecraft>>| {
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
         parts: Query<&ChildOf, With<PartInstance>>| {
            if let Ok(child_of) = parts.get(trigger.target()) {
                commands.entity(child_of.0).insert(Selected);
            }
            trigger.propagate(false);
        },
    );

    commands.spawn((
        Transform::default().rotate_z(30.0f32.to_radians()),
        Collider::from(floor),
        RigidBody::Static,
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
    spacecraft: Query<&GlobalTransform, With<Spacecraft>>,
    parts: Query<&PartInstance>,
    mut sprites: Query<(&mut Sprite, &ChildOf), With<PartSprite>>,
    camera: Query<(&Camera, &GlobalTransform)>,
) -> Result {
    let (camera, transform) = camera.single()?;
    for event in events.read() {
        info!("Spacecraft event: {:?}", event);

        match event {
            SpacecraftEvent::Spawn { name, pos } => {
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

                let name = format!("sc-{}", name);
                spawn_spacecraft(
                    &mut commands,
                    name,
                    *pos,
                    &vehicle,
                    &mut meshes,
                    &mut asset_server,
                    &args,
                );
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
            SpacecraftEvent::ToggleView(real) => {
                for (mut sprite, child_of) in &mut sprites {
                    let color = match *real {
                        SpacecraftView::Damage => RED,
                        SpacecraftView::Real => WHITE,
                        SpacecraftView::Type => {
                            if let Ok(part) = parts.get(child_of.0) {
                                let c = starling::vehicle::diagram_color(&part.0);
                                Srgba::from_f32_array(c)
                            } else {
                                BLACK
                            }
                        }
                    };
                    sprite.color = color.into();
                }
            }
        }
    }

    Ok(())
}

fn spawn_spacecraft(
    commands: &mut Commands,
    name: impl Into<String>,
    pos: Vec2,
    vehicle: &Vehicle,
    meshes: &mut ResMut<Assets<Mesh>>,
    asset_server: &mut ResMut<AssetServer>,
    args: &Res<ProgramContext>,
) {
    let name = name.into();

    let r = vehicle.bounding_radius();

    commands
        .spawn((
            Name::new(name.clone()),
            Transform::from_translation(pos.extend(0.0)),
            Spacecraft,
            RigidBody::Dynamic,
            Mass(vehicle.total_mass().to_kg_f64() as f32),
            Collider::rectangle(r as f32 * 1.5, r as f32 * 0.7),
            Visibility::default(),
            AngularVelocity(rand(-0.3, 0.3)),
            children![
                (
                    Transform::from_translation(Vec3::new(0.0, 0.0, 2.0))
                        .with_scale(Vec3::splat(0.005)),
                    Text2d::new(name.to_uppercase()),
                    TextFont::from_font_size(64.0),
                    TextColor(WHITE.with_alpha(0.7).into()),
                ),
                (
                    Transform::from_translation(Vec3::new(-0.02, -0.02, 0.1))
                        .with_scale(Vec3::splat(0.005)),
                    Text2d::new(name.to_uppercase()),
                    TextFont::from_font_size(64.0),
                    TextColor(BLACK.into())
                ),
            ],
        ))
        .with_children(|parent| {
            for (_, part) in vehicle.parts() {
                let dims = part.prototype().dims_meters();
                let dims_rot = part.dims_meters();
                let origin = part.origin_meters() + dims_rot / 2.0;

                let (z, alpha, t, d) = match part.layer() {
                    starling::parts::PartLayer::Internal => (0.0, 1.0, 0.5, 0.0),
                    starling::parts::PartLayer::Plumbing => continue,
                    starling::parts::PartLayer::Structural => (0.02, 0.7, 0.7, 0.05),
                    starling::parts::PartLayer::Exterior => (0.04, 0.2, 0.8, 0.1),
                };

                let dims = dims - d;
                let polygon = Rectangle::new(dims.x, dims.y);

                let color = starling::prelude::diagram_color(&part.prototype());
                let color = Srgba::from_f32_array(color).with_alpha(alpha);

                let path = args.part_sprite_path(part.prototype().part_name());
                let sprite = asset_server.load(path);

                let name = part.prototype().part_name().to_string();

                let occludes = part.prototype().layer() == starling::parts::PartLayer::Internal;
                let emits_light = part.as_thruster().map(|(t, _)| t.is_rcs()).unwrap_or(false);

                let mut sprite = Sprite::from_image(sprite);
                sprite.color = WHITE.mix(&BLACK, 0.5).into();

                parent
                    .spawn((
                        Name::new(format!("Part ({})", name)),
                        Transform::from_translation(origin.extend(z))
                            .with_scale(Vec3::splat(1.0))
                            .with_rotation(
                                Quat::from_rotation_z(part.rotation().to_angle() as f32),
                            ),
                        // the mesh is for picking!
                        Mesh2d(meshes.add(polygon)),
                        // MeshMaterial2d(materials.add(Color::from(color))),
                        // Mesh3d(meshes.add(Cuboid::new(dims.x, dims.y, t))),
                        // MeshMaterial3d(std_mat.add(Color::Srgba(color))),
                        PartInstance(part.prototype()),
                    ))
                    .with_child((
                        PartSprite,
                        sprite,
                        Transform::from_scale(Vec3::splat(1.0 / 20.0)),
                    ))
                    .insert_if(
                        LightOccluder2d {
                            shape: LightOccluder2dShape::Rectangle {
                                half_size: dims_rot / 2.0,
                            },
                        },
                        || occludes && !emits_light,
                    )
                    .insert_if(
                        crate::new::particles::ParticleEmitter {
                            enabled: true,
                            size: Vec3::splat(0.1),
                        },
                        || rand(0.0, 1.0) < 0.01,
                    )
                    .insert_if(
                        (
                            ShipLight(rand(0.1, 1.0)),
                            PointLight2d {
                                intensity: 20.0,
                                radius: 1.0,
                                color: RED.into(),
                                ..default()
                            },
                        ),
                        || emits_light,
                    );
            }
        });
}
