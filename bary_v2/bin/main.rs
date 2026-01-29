use bary_core::prelude::*;
use bary_v1::args::ProgramContext;
use bary_v2::*;
use bevy::{
    post_process::bloom::Bloom,
    sprite_render::{Wireframe2dConfig, Wireframe2dPlugin},
};
use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings};

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
        .add_plugins(plot_plugin)
        .add_systems(EguiPrimaryContextPass, egui_ui)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_wireframe,
                save_settings_on_change.run_if(on_timer(std::time::Duration::from_secs(1))),
                toggle_inv_on_alt,
            )
                .chain(),
        )
        .add_systems(Startup, update_gizmo_config)
        .run();
}

fn update_wireframe(mut wireframe_config: ResMut<Wireframe2dConfig>, settings: Res<Settings>) {
    wireframe_config.global = settings.show_wireframes;
}

fn update_gizmo_config(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = 6.0;
}

#[derive(Resource)]
pub struct ShipNames(pub Vec<String>);

pub fn load_names_from_file(filename: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(filename)?
        .lines()
        .filter_map(|s| (!s.is_empty()).then(|| s.to_string()))
        .collect())
}

pub fn get_random_ship_name(names: &Vec<String>) -> String {
    if names.is_empty() {
        return String::new();
    }
    let idx = randint(0, names.len() as i32) as usize;
    names[idx].clone()
}

fn setup(mut commands: Commands) -> Result {
    let ctx = ProgramContext::default();
    let settings = Settings::from_file(&ctx.settings_path()).unwrap_or(Settings::default());
    let parts = load_parts_from_dir_2(&ctx).unwrap_or(PartDatabase::default());
    let save_data = match SaveData::from_file(&ctx.save_data_path()) {
        Ok(sd) => sd,
        Err(e) => {
            error!("Failed to load save data: {}", e);
            SaveData::default()
        }
    };

    let ship_names = load_names_from_file(&ctx.names_path()).unwrap_or(vec![]);

    commands.insert_resource(PartsResource(parts));
    commands.insert_resource(ctx);
    commands.insert_resource(settings);
    commands.insert_resource(ClearColor(BLACK.into()));

    commands.spawn((
        Camera2d::default(),
        Camera::default(),
        Transform::from_xyz(0.0, 20.0, 0.0).with_scale(Vec3::splat(0.1)),
        Bloom {
            intensity: 0.2,
            ..Bloom::OLD_SCHOOL
        },
    ));

    for ship in save_data.ships {
        info!("Spawning {}", &ship.name);
        commands.trigger(SpacecraftEvent::SpawnVehicle {
            blueprint_name: ship.name,
            ship_name: get_random_ship_name(&ship_names),
            pos: ship.pos,
            angle: ship.angle,
        });
    }

    commands.insert_resource(ShipNames(ship_names));

    Ok(())
}

fn toggle_inv_on_alt(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<Settings>) {
    if keys.just_pressed(KeyCode::AltLeft) {
        settings.draw_inventories = !settings.draw_inventories;
    }
}
