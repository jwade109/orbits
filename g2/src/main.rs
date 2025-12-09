mod game_version_two;

use crate::game_version_two::*;

use avian2d::prelude::*;
use bevy::core_pipeline::bloom::Bloom;
use game::args::ProgramContext;

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
        // .insert_gizmo_config(
        //     PhysicsGizmos {
        //         aabb_color: Some(Color::WHITE),
        //         ..default()
        //     },
        //     GizmoConfig::default(),
        // )
        // 3rd-party plugins
        // .add_plugins(MeshPickingPlugin)
        .add_plugins(Wireframe2dPlugin::default())
        .add_plugins(EguiPlugin::default())
        // .add_plugins(WorldInspectorPlugin::new())
        .add_systems(EguiPrimaryContextPass, egui_ui)
        .add_plugins(Shape2dPlugin::default())
        .add_plugins(ThrusterPlugin::default())
        // plugins I've implemented
        .add_plugins(ParticlePlugin)
        .add_plugins(AnimatedTextPlugin)
        .add_plugins(SpacecraftPlugin)
        .add_plugins(ComputerPlugin)
        .add_plugins(TerrainPlugin)
        .add_plugins(CursorPlugin)
        .add_plugins(CameraPlugin)
        .add_systems(Startup, (setup, toggle_wireframe))
        .run();
}

fn toggle_wireframe(mut wireframe_config: ResMut<Wireframe2dConfig>) {
    // wireframe_config.global = !wireframe_config.global;
}

fn setup(mut commands: Commands) -> Result {
    commands.insert_resource(ProgramContext::default());

    commands.insert_resource(ClearColor(BLACK.into()));

    commands.insert_resource(Gravity(Vec2::ZERO));

    commands.spawn((
        Camera2d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Transform::from_xyz(0.0, 20.0, 0.0).with_scale(Vec3::splat(0.1)),
        Bloom {
            intensity: 0.2,
            ..Bloom::OLD_SCHOOL
        },
    ));

    commands.send_event(SpacecraftEvent::SpawnVehicle {
        name: "miner".to_string(),
        pos: Vec2::new(20.0, 20.0),
        angle: rand(-0.2, 0.3),
    });

    for _ in 0..4 {
        for name in [
            "pollux",
            "remora",
            "remora",
            "remora",
            "remora",
            "remora",
            // "bellerophon",
            // "lander",
            // "remora",
            // "icecream",
            // "spacestation",
        ] {
            let x = rand(-20.0, 20.0);
            let y = rand(10.0, 30.0);
            commands.send_event(SpacecraftEvent::SpawnVehicle {
                name: name.to_string(),
                pos: Vec2::new(x, y),
                angle: rand(-0.2, 0.3),
            });
        }
    }

    Ok(())
}
