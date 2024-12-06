
use bevy::prelude::*;

mod balls;
mod debug;
mod player;
mod bounded;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(crate::debug::DebugPlugin {})
        .add_plugins(crate::balls::BallsPlugin {})
        .add_plugins(crate::player::SpaceshipPlugin {})
        .run();
}
