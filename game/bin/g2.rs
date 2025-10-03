use avian2d::prelude::*;
use bevy::color::palettes::css::*;
use bevy::core_pipeline::bloom::Bloom;
use bevy::pbr::{CascadeShadowConfigBuilder, NotShadowCaster};
use bevy::prelude::*;
use bevy::sprite::{Wireframe2dConfig, Wireframe2dPlugin};
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy_egui::EguiPlugin;
use bevy_fly_camera::FlyCameraPlugin;
use bevy_inspector_egui::quick::*;
use clap::Parser;
use game::args::ProgramContext;
use starling::prelude::Vehicle;
use std::fmt::Debug;
use game::animated_text::AnimatedTextPlugin;

fn cursor_grab(
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut app_exit_events: ResMut<Events<bevy::app::AppExit>>,
) {
    let mut primary_window = q_windows.single_mut().unwrap();

    // if you want to use the cursor, but not let it leave the window,
    // use `Confined` mode:
    primary_window.cursor_options.grab_mode = CursorGrabMode::None;

    if keys.just_pressed(KeyCode::Escape) {
        app_exit_events.send(bevy::app::AppExit::Success);
    }
}

fn random(min: f32, max: f32) -> f32 {
    use rand::Rng;
    rand::rng().random_range(min..=max)
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    unapproved_path_mode: bevy::asset::UnapprovedPathMode::Allow,
                    ..default()
                }),
        )
        .add_plugins(MeshPickingPlugin)
        .add_plugins(Wireframe2dPlugin::default())
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(FlyCameraPlugin)
        .insert_resource(ThrustParticleConfig::default())
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(ResourceInspectorPlugin::<ThrustParticleConfig>::new())
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_wireframe, draw_collider_aabbs, cursor_grab))
        .add_systems(FixedUpdate, thrust_particles)
        .add_plugins(AnimatedTextPlugin)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut asset_server: ResMut<AssetServer>,
    mut std_mat: ResMut<Assets<StandardMaterial>>,
) -> Result {
    commands.spawn((
        PointLight {
            intensity: 10000000000000.0,
            color: WHITE.mix(&YELLOW, 0.3).into(),
            radius: 10.0,
            range: 100000.0,
            ..default()
        },
        Transform::from_translation(Vec3::splat(11000.0).with_z(1000.0)),
    ));

    let camera_pos = Vec3::new(0.0, 200.0, 300.0);

    commands.spawn((
        Camera3d::default(),
        // Exposure::SUNLIGHT,
        Bloom {
            intensity: 0.4,
            ..Bloom::NATURAL
        },
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.1,
            maximum_distance: 10_000.0,
            first_cascade_far_bound: 100.0,
            overlap_proportion: 0.2,
        }
        .build(),
        // Projection::Orthographic(OrthographicProjection::default_3d()),
        // Transform::from_translation(camera_pos).looking_at(camera_pos.with_z(0.0), Vec3::Y),
        Transform::from_translation(Vec3::new(20.0, 20.0, 5.0)).looking_at(Vec3::Z * 12.0, Vec3::Y),
        // FlyCamera::default(),
    ));

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

    let args = ProgramContext::try_parse().unwrap_or(ProgramContext::default());

    commands.insert_resource(ClearColor(BLACK.into()));
    commands.insert_resource(Gravity(Vec2::ZERO));

    // commands.spawn(
    //     AmbientLight {
    //         color: WHITE.into(),
    //         brightness: 1000.0,
    //         ..default()
    //     },
    // );

    commands.spawn((ParticleEmitter { enabled: true }, Name::new("Emitter 1")));
    commands.spawn((
        ParticleEmitter { enabled: true },
        Transform::from_xyz(-13.0, 0.0, 4.0),
        Name::new("Emitter 2"),
    ));

    let floor = Rectangle::new(5000.0, 5.0);

    commands.spawn((
        Transform::default().rotate_z(30.0f32.to_radians()),
        Collider::from(floor),
        RigidBody::Static,
        Mesh2d(meshes.add(floor)),
        MeshMaterial2d(materials.add(Color::from(GRAY.with_alpha(0.4)))),
    ));

    for _ in 0..10 {
        for name in [
            "pollux",
            "remora",
            "bellerophon",
            "lander",
            "remora",
            "icecream",
        ] {
            let vehicle_path = args.vehicle_dir().join(format!("{name}.vehicle"));
            let parts = starling::vehicle::load_parts_from_dir(&args.parts_dir())?;
            let vehicle = starling::vehicle::load_vehicle(&vehicle_path, "".to_string(), &parts)
                .map_err(|_| "bad vehicle path")?;

            let name = format!("sc-{}", name);
            let x = random(-200.0, 200.0);
            let y = random(100.0, 300.0);
            spacecraft(
                &mut commands,
                name,
                x,
                y,
                0.0,
                &vehicle,
                &mut meshes,
                &mut materials,
                &mut asset_server,
                &mut std_mat,
                &args,
            );
        }
    }

    Ok(())
}

fn draw_collider_aabbs(
    mut gizmos: Gizmos,
    query: Query<&ColliderAabb>,
    wireframe_config: Res<Wireframe2dConfig>,
) {
    if !wireframe_config.global {
        return;
    }
    for aabb in &query {
        let size = aabb.max - aabb.min;
        let center = size / 2.0 + aabb.min;
        let iso = Isometry3d::from_translation(center.extend(0.0));
        gizmos.rect(iso, size, RED);
    }
}

fn toggle_wireframe(
    mut wireframe_config: ResMut<Wireframe2dConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // if keyboard.just_pressed(KeyCode::Space) {
    //     wireframe_config.global = !wireframe_config.global;
    // }
}

#[derive(Component, Debug)]
struct Spacecraft;

#[derive(Component, Debug)]
struct PartInstance;

#[derive(Component, Debug)]
#[require(Transform, NotShadowCaster)]
struct ThrustParticle {
    time_remaining: f32,
    velocity: Vec3,
    nominal_position: Vec3,
}

#[derive(Component, Debug, Reflect)]
#[require(Transform)]
struct ParticleEmitter {
    enabled: bool,
}

#[derive(Resource, Reflect, Debug)]
struct ThrustParticleConfig {
    color_a: Color,
    color_b: Color,
    mean_velocity: f32,
    velocity_spread: f32,
    spread: f32,
    discrete: bool,
    paused: bool,
    step: bool,
    cuboids: bool,
}

impl Default for ThrustParticleConfig {
    fn default() -> Self {
        ThrustParticleConfig {
            color_a: RED.into(),
            color_b: YELLOW.into(),
            mean_velocity: 15.0,
            velocity_spread: 3.0,
            spread: 1.0,
            discrete: false,
            paused: false,
            step: false,
            cuboids: true,
        }
    }
}

fn thrust_particles(
    mut commands: Commands,
    emitters: Query<(&GlobalTransform, &ParticleEmitter)>,
    mut query: Query<(Entity, &mut ThrustParticle, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_mat: ResMut<Assets<StandardMaterial>>,
    mut cfg: ResMut<ThrustParticleConfig>,
    time_fixed: Res<Time<Fixed>>,
    mut gizmos: Gizmos,
) {
    if cfg.paused && !cfg.step {
        return;
    }

    cfg.step = false;

    let discrete_n = 3;
    let particle_size = 1.0 / discrete_n as f32;

    use starling::prelude::{rotate, PI};

    for (tf, emitter) in &emitters {
        gizmos.primitive_3d(
            &Cuboid::from_length(2.0),
            Isometry3d::from_translation(tf.translation()),
            RED,
        );

        if !emitter.enabled {
            continue;
        }

        for _ in 0..8 {
            let angle = random(0.0, 2.0 * PI);
            let cross = rotate(Vec2::X, angle) * random(0.2, 1.0) * cfg.spread;
            let vel = cross.extend(random(-1.0, 1.0) * cfg.velocity_spread + cfg.mean_velocity);
            let color = cfg.color_a.mix(&cfg.color_b, random(0.1, 0.9));

            let x = random(-1.0, 1.0) * 1.0;
            let y = random(-1.0, 1.0) * 1.0;
            let z = random(-1.0, 1.0) * 1.0;

            let pos = Vec3::new(x, y, z) + tf.translation();

            let dpos = if cfg.discrete {
                (pos * discrete_n as f32).round() / discrete_n as f32
            } else {
                pos
            };

            commands.spawn((
                ThrustParticle {
                    time_remaining: random(0.6, 1.2),
                    velocity: vel,
                    nominal_position: pos,
                },
                Transform::from_translation(dpos),
                if cfg.cuboids {
                    Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(particle_size) * 2.0)))
                } else {
                    Mesh3d(meshes.add(Sphere::new(particle_size)))
                },
                MeshMaterial3d(std_mat.add(color)),
            ));
        }
    }

    let dt = time_fixed.delta_secs();

    for (e, mut part, mut tf) in &mut query {
        part.time_remaining -= dt;
        part.nominal_position = part.nominal_position + part.velocity * dt;
        if cfg.discrete {
            tf.translation =
                (part.nominal_position * discrete_n as f32).round() / discrete_n as f32;
        } else {
            tf.translation = part.nominal_position;
        }
        if part.time_remaining < 0.0 {
            commands.entity(e).despawn();
        }
    }
}

fn spacecraft(
    commands: &mut Commands,
    name: impl Into<String>,
    x: f32,
    y: f32,
    z: f32,
    vehicle: &Vehicle,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    asset_server: &mut ResMut<AssetServer>,
    std_mat: &mut ResMut<Assets<StandardMaterial>>,
    args: &ProgramContext,
) {
    let name = name.into();

    let r = vehicle.bounding_radius();

    commands
        .spawn((
            Name::new(name.clone()),
            Transform::from_translation(Vec3::new(x, y, z)),
            Spacecraft,
            RigidBody::Dynamic,
            Mass(vehicle.total_mass().to_kg_f64() as f32),
            Collider::rectangle(r as f32 * 1.5, r as f32 * 0.7),
            Visibility::default(),
            AngularVelocity(random(-0.3, 0.3)),
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
                )
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
                    starling::parts::PartLayer::Structural => (0.0, 0.7, 0.7, 0.05),
                    starling::parts::PartLayer::Exterior => (0.0, 0.2, 0.8, 0.1),
                };

                let dims = dims - d;
                let polygon = Rectangle::new(dims.x, dims.y);

                let color = starling::prelude::diagram_color(&part.prototype());
                let color = Srgba::from_f32_array(color).with_alpha(alpha);

                let path = args.part_sprite_path(part.prototype().part_name());
                // let sprite = asset_server.load(path);

                parent.spawn((
                    Name::new(part.prototype().part_name().to_string()),
                    Transform::from_translation(origin.extend(z))
                        .with_scale(Vec3::splat(1.0))
                        .with_rotation(Quat::from_rotation_z(part.rotation().to_angle() as f32)),
                    // the mesh is for picking!
                    Mesh2d(meshes.add(polygon)),
                    // MeshMaterial2d(materials.add(Color::from(color))),
                    Mesh3d(meshes.add(Cuboid::new(dims.x, dims.y, t))),
                    MeshMaterial3d(std_mat.add(Color::Srgba(color))),
                    PartInstance,
                ));
                // .with_child((
                //     Sprite::from_image(sprite),
                //     Transform::from_scale(Vec3::splat(1.0 / 20.0)),
                // ));
            }
        });
}
