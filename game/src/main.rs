// #![windows_subsystem = "windows"]

use bevy::prelude::*;

mod drawing;
mod graph;
mod keybindings;
mod mouse;
mod notifications;
mod planetary;
mod scenes;
mod sprites;
mod ui;

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
        .add_plugins(crate::planetary::PlanetaryPlugin {})
        .add_plugins(crate::sprites::SpritePlugin {})
        .add_plugins(crate::ui::UiPlugin {})
        .run();
}
