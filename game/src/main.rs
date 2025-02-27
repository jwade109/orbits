use bevy::prelude::*;

mod button;
mod camera_controls;
mod craft;
mod debug;
mod drawing;
mod grid_sprites;
mod planetary;
mod sprites;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(crate::debug::DebugPlugin {})
        .add_plugins(crate::planetary::PlanetaryPlugin {})
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
