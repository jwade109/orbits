pub mod assets;
pub mod camera;
pub mod constants;
pub mod editor_state;
pub mod imgui;
pub mod render;
pub mod sim;
pub mod sounds;
pub mod ui;
pub mod utils;

mod commands;
pub mod headless_server;
mod new_ui;

pub use commands::*;
pub use new_ui::*;

/// A namespace of functions which perform operations in the simulation world
/// and maintain certain invariants. Using these is safer than performing
/// direct manipulation of world entities, though potentially costlier
/// since using several in succession might perform duplicate lookups.
pub use crate::sim::set_all_thrusters;
pub use crate::sim::update_grid_acceleration;
