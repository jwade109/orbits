mod game_version_two;

use crate::game_version_two::*;

use bevy::core_pipeline::bloom::Bloom;
use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings};
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
        .edit_schedule(Update, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        })
        .add_plugins(Wireframe2dPlugin::default())
        .add_plugins(EguiPlugin::default())
        // .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(Shape2dPlugin::default())
        .add_plugins(ThrusterPlugin::default())
        // plugins I've implemented
        .add_plugins(ParticlePlugin)
        .add_plugins(AnimatedTextPlugin)
        .add_plugins(SpacecraftPlugin)
        .add_plugins(ComputerPlugin)
        .add_plugins(TerrainPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(InventoryTransferPlugin)
        .add_systems(EguiPrimaryContextPass, egui_ui)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_wireframe,
                save_settings_on_change.run_if(on_timer(std::time::Duration::from_secs(1))),
                toggle_inv_on_alt,
            )
                .chain()
                .in_set(Sets::Misc),
        )
        .configure_sets(
            Update,
            (
                Sets::Input,
                Sets::PrePhysics,
                Sets::Physics,
                Sets::PostPhysics,
                Sets::Misc,
            )
                .chain(),
        )
        .configure_sets(
            FixedUpdate,
            (
                Sets::Input,
                Sets::PrePhysics,
                Sets::Physics,
                Sets::PostPhysics,
                Sets::Misc,
            )
                .chain(),
        )
        .configure_sets(
            PostUpdate,
            (Sets::Draw, Sets::PostPhysics).after(TransformSystem::TransformPropagate),
        )
        .run();
}

fn update_wireframe(mut wireframe_config: ResMut<Wireframe2dConfig>, settings: Res<Settings>) {
    wireframe_config.global = settings.show_wireframes;
}

fn setup(mut commands: Commands) -> Result {
    let ctx = ProgramContext::default();
    let settings = Settings::from_file(&ctx.settings_path()).unwrap_or(Settings::default());
    let parts = load_parts_from_dir(&ctx).unwrap_or(PartDatabase::default());

    commands.insert_resource(parts);
    commands.insert_resource(ctx);
    commands.insert_resource(settings);
    commands.insert_resource(ClearColor(BLACK.into()));

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

    let off = Vec2::splat(20.0);

    // commands.send_event(SpacecraftEvent::SpawnVehicle {
    //     name: "remora".to_string(),
    //     pos: off + Vec2::ZERO,
    //     angle: 0.0,
    // });

    // commands.send_event(SpacecraftEvent::SpawnVehicle {
    //     name: "pollux".to_string(),
    //     pos: off + Vec2::X * 4.0,
    //     angle: 1.57,
    // });

    commands.send_event(SpacecraftEvent::SpawnVehicle {
        name: "miner".to_string(),
        pos: off + Vec2::splat(4.0),
        angle: 1.57,
    });

    for name in [
        "pollux",
        "pollux",
        "remora",
        "bellerophon",
        // "lander",
        // "remora",
        // "icecream",
        // "spacestation",
        // "remora",
        // "remora",
        // "remora",
        // "remora",
        // "remora",
        // "foundation",
        // "miner",
    ] {
        let x = rand(-100.0, 100.0);
        let y = rand(-100.0, 100.0);
        commands.send_event(SpacecraftEvent::SpawnVehicle {
            name: name.to_string(),
            pos: Vec2::new(x + 25.0, y + 25.0),
            angle: rand(-0.2, 0.3),
        });
    }

    Ok(())
}

fn toggle_inv_on_alt(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<Settings>) {
    if keys.just_pressed(KeyCode::AltLeft) {
        settings.draw_inventories = !settings.draw_inventories;
    }
}
