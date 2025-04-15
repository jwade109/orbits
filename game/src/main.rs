// #![windows_subsystem = "windows"]

use bevy::prelude::*;

mod camera_controls;
mod drawing;
mod graph;
mod inventory;
mod keybindings;
mod mouse;
mod notifications;
mod planetary;
mod scene;
mod sprites;
mod ui;

fn main() {
    let mut window = Window {
        // mode: bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        title: "Space UPS".into(),
        ..default()
    };

    window.set_maximized(true);

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
