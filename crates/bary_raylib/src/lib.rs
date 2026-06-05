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
pub mod world_builder;

mod commands;
pub mod headless_server;
mod new_ui;

pub use commands::*;
pub use new_ui::*;

/// A namespace of functions which are potentially expensive,
/// but nonetheless offer a readonly way to retrieve some information
/// of interest. Some of these are acceptable to run during sims;
/// others should only be used to verify correctness in tests.
pub use crate::sim::get_blueprint_by_id;
pub use crate::sim::get_closest_grid;
pub use crate::sim::get_grid_by_name;
pub use crate::sim::get_grid_origin;
pub use crate::sim::get_sum_linear_forces;
pub use crate::sim::sum_part_masses;
pub use crate::sim::sum_part_masses_w;

/// A namespace of functions which perform operations in the simulation world
/// and maintain certain invariants. Using these is safer than performing
/// direct manipulation of world entities, though potentially costlier
/// since using several in succession might perform duplicate lookups.
pub use crate::sim::despawn_grid;
pub use crate::sim::set_all_thrusters;
pub use crate::sim::set_primary_computer_state;
pub use crate::sim::set_primary_computer_state_c;
pub use crate::sim::set_primary_computer_waypoint;
pub use crate::sim::set_primary_computer_waypoint_c;
pub use crate::sim::update_grid_acceleration;
pub use crate::sim::update_world;
