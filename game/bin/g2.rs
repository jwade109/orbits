use avian2d::prelude::*;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy::sprite::{Wireframe2dConfig, Wireframe2dPlugin};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_pancam::{DirectionKeys, PanCam, PanCamPlugin};
use clap::Parser;
use starling::prelude::Vehicle;
use std::fmt::Debug;

use game::args::ProgramContext;

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
        .add_plugins(PanCamPlugin::default())
        .add_plugins(Wireframe2dPlugin::default())
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        .add_systems(Startup, setup)
        .add_systems(Update, (update, toggle_wireframe, draw_collider_aabbs))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut asset_server: ResMut<AssetServer>,
) -> Result {
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

    commands.spawn((
        Camera2d,
        PanCam {
            grab_buttons: vec![MouseButton::Left, MouseButton::Middle], // which buttons should drag the camera
            move_keys: DirectionKeys::NONE,
            speed: 400.,              // the speed for the keyboard movement
            enabled: true,            // when false, controls are disabled. See toggle example.
            zoom_to_cursor: true, // whether to zoom towards the mouse or the center of the screen
            min_scale: 0.01,      // prevent the camera from zooming too far in
            max_scale: 40.,       // prevent the camera from zooming too far out
            min_x: f32::NEG_INFINITY, // minimum x position of the camera window
            max_x: f32::INFINITY, // maximum x position of the camera window
            min_y: f32::NEG_INFINITY, // minimum y position of the camera window
            max_y: f32::INFINITY, // maximum y position of the camera window
        },
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
                &args,
            );
        }
    }

    Ok(())
}

fn update(mut query: Query<&mut PanCam>, keyboard: Res<ButtonInput<KeyCode>>) {
    for mut cam in &mut query {
        cam.enabled = keyboard.pressed(KeyCode::ShiftLeft);
    }
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
    if keyboard.just_pressed(KeyCode::Space) {
        wireframe_config.global = !wireframe_config.global;
    }
}

#[derive(Component, Debug)]
struct Spacecraft;

#[derive(Component, Debug)]
struct PartInstance(pub starling::prelude::InstantiatedPart);

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
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.11))
                        .with_scale(Vec3::splat(0.005)),
                    Text2d::new(name.to_uppercase()),
                    TextFont::from_font_size(64.0),
                    TextColor(WHITE.with_alpha(0.7).into())
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
                let dims = dims - 0.02;

                let color = starling::prelude::diagram_color(&part.prototype());
                let color = Srgba::from_f32_array(color);

                let polygon = Rectangle::new(dims.x, dims.y);

                let (z, alpha) = match part.layer() {
                    starling::parts::PartLayer::Internal => (0.001, 1.0),
                    starling::parts::PartLayer::Plumbing => (0.002, 1.0),
                    starling::parts::PartLayer::Structural => (0.003, 0.3),
                    starling::parts::PartLayer::Exterior => (0.004, 0.2),
                };

                let path = args.part_sprite_path(part.prototype().part_name());
                let sprite = asset_server.load(path);

                parent
                    .spawn((
                        Name::new(part.prototype().part_name().to_string()),
                        Transform::from_translation(origin.extend(z))
                            .with_scale(Vec3::splat(1.0))
                            .with_rotation(
                                Quat::from_rotation_z(part.rotation().to_angle() as f32),
                            ),
                        // the mesh is for picking!
                        Mesh2d(meshes.add(polygon)),
                        // MeshMaterial2d(materials.add(Color::from(color.with_alpha(alpha)))),
                        PartInstance(part.clone()),
                    ))
                    .with_child((
                        Sprite::from_image(sprite),
                        Transform::from_scale(Vec3::splat(1.0 / 20.0)),
                    ));
            }
        });
}
