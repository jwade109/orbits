// #![windows_subsystem = "windows"]

use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;

fn main() {
    let window = Window {
        mode: bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        title: "Space UPS".into(),
        ..default()
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(window),
            ..default()
        }))
        .add_plugins(Shape2dPlugin::default())
        .add_plugins(game::game::GamePlugin {})
        .add_plugins(game::ui::UiPlugin {})
        .run();
}
