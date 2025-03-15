use bevy::prelude::*;

mod camera_controls;
mod debug;
mod drawing;
mod keybindings;
mod mouse;
mod planetary;
mod sprites;
mod ui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(crate::debug::DebugPlugin {})
        .add_plugins(crate::planetary::PlanetaryPlugin {})
        .add_plugins(crate::sprites::SpritePlugin {})
        .add_plugins(crate::ui::UiPlugin {})
        .run();
}
