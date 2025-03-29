// #![windows_subsystem = "windows"]

use bevy::prelude::*;

mod camera_controls;
mod debug;
mod drawing;
mod egui;
mod graph;
mod keybindings;
mod mouse;
mod notifications;
mod planetary;
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
        .add_plugins(crate::debug::DebugPlugin {})
        .add_plugins(crate::planetary::PlanetaryPlugin {})
        .add_plugins(bevy_egui::EguiPlugin)
        .add_plugins(crate::sprites::SpritePlugin {})
        .add_plugins(crate::ui::UiPlugin {})
        .run();
}
