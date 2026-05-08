use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WireframePlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (update_wireframe, update_wave_meshes, spin_camera))
        .run();
}

fn spin_camera(mut camera: Single<&mut Transform, With<Camera3d>>, time: Res<Time>) {
    let r = 8.0;
    let t = time.elapsed_secs();
    let a = t * 0.05;
    camera.translation.x = a.cos() * r;
    camera.translation.z = a.sin() * r;

    camera.look_at(Vec3::ZERO, Vec3::Y);
}

fn wave_func(x: f32, z: f32, t: f32) -> f32 {
    let w1 = (x + z * 0.5 + t).sin() * 0.4;
    let w2 = (-x * 0.2 + z * 0.6 + t).cos() * 0.3;
    let d = Vec2::new(x, z).length();
    let w3 = d.cos() / (d + 2.0) * 2.0;
    w1 + w2 + w3
}

#[derive(Component)]
pub struct WaveMesh;

fn update_wave_meshes(meshes: Query<&mut Transform, With<WaveMesh>>, time: Res<Time>) {
    let t = time.elapsed_secs();
    for mut transform in meshes {
        let p = &mut transform.translation;
        p.y = wave_func(p.x, p.z, t);
    }
}

fn update_wireframe(
    keys: Res<ButtonInput<KeyCode>>,
    mut wireframe_config: ResMut<WireframeConfig>,
) {
    if keys.just_pressed(KeyCode::Space) {
        wireframe_config.global ^= true;
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // circular base

    let s = 10.0;
    let mesh = meshes.add(Cuboid::from_length(1.0 / s * 1.1).mesh());

    for x in -100..=100 {
        for z in -100..=100 {
            let x = x as f32 / s;
            let z = z as f32 / s;
            let transform = Transform::from_xyz(x, 0.0, z)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::PI / 2.0));
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(materials.add(Color::srgb_u8(200, 200, 255))),
                transform,
                WaveMesh,
            ));
        }
    }

    let monkey_handle = asset_server.load("Monkey.gltf#Mesh0/Primitive0");

    // cube
    // commands.spawn((
    //     Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
    //     MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
    //     Transform::from_xyz(0.0, 0.5, 0.0),
    // ));

    // monkey
    commands.spawn((
        Mesh3d(monkey_handle),
        MeshMaterial3d(materials.add(Color::srgb_u8(255, 144, 150))),
        Transform::from_xyz(0.0, 2.0, 0.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 2.0)),
    ));

    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
