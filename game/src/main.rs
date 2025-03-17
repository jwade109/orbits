use bevy::prelude::*;

mod camera_controls;
mod debug;
mod drawing;
mod egui;
mod keybindings;
mod mouse;
mod planetary;
mod sprites;
mod ui;
mod warnings;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(crate::debug::DebugPlugin {})
        .add_plugins(crate::planetary::PlanetaryPlugin {})
        .add_plugins(bevy_egui::EguiPlugin)
        .add_plugins(crate::sprites::SpritePlugin {})
        .add_plugins(crate::ui::UiPlugin {})
        .add_plugins(crate::warnings::WarningsPlugin {})
        .run();
}
