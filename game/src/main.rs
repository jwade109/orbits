use bevy::prelude::*;

mod craft;
mod debug;
mod drawing;
mod planetary;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(crate::debug::DebugPlugin {})
        // .add_plugins(crate::balls::BallsPlugin {})
        // .add_plugins(crate::player::SpaceshipPlugin {})
        // .add_plugins(crate::planetary::PlanetaryPlugin {})
        .add_plugins(crate::craft::CraftPlugin {})
        .add_systems(Startup, (setup, query_camera).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn query_camera(mut query: Query<&mut Transform, With<Camera>>) {
    for mut cam in query.iter_mut() {
        cam.scale *= 6.5;
    }
}
