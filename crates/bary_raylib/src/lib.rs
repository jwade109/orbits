pub mod assets;
pub mod camera;
pub mod constants;
pub mod editor_state;
pub mod imgui;
pub mod persistence;
pub mod render;
pub mod sim;
pub mod sounds;
pub mod tests;
pub mod ui;
pub mod utils;
pub mod world_builder;

mod commands;

pub use commands::*;

/// A namespace of functions which are potentially expensive,
/// but nonetheless offer a readonly way to retrieve some information
/// of interest. Some of these are acceptable to run during sims;
/// others should only be used to verify correctness in tests.
pub use crate::sim::get_blueprint_by_id;
pub use crate::sim::get_closest_grid;
pub use crate::sim::get_grid_by_name;
pub use crate::sim::get_grid_origin;
pub use crate::sim::get_grid_physical_props_by_id;
pub use crate::sim::get_primary_cpu_id;
pub use crate::sim::get_proto_by_name;
pub use crate::sim::get_sum_linear_forces;
pub use crate::sim::sum_part_masses;
pub use crate::sim::sum_part_masses_w;

/// A namespace of functions which perform operations in the simulation world
/// and maintain certain invariants. Using these is safer than performing
/// direct manipulation of world entities, though potentially costlier
/// since using several in succession might perform duplicate lookups.
pub use crate::sim::despawn_all_vehicles;
pub use crate::sim::despawn_grid;
pub use crate::sim::destroy_part_without_integrity_check;
pub use crate::sim::detach_part_from_parent;
pub use crate::sim::duplicate_part_to_new_grid;
pub use crate::sim::insert_part;
pub use crate::sim::insert_part_c;
pub use crate::sim::rebuild_index_from_island;
pub use crate::sim::set_all_thrusters;
pub use crate::sim::set_grid_pose;
pub use crate::sim::set_grid_vel;
pub use crate::sim::set_primary_computer_state;
pub use crate::sim::set_primary_computer_state_c;
pub use crate::sim::set_primary_computer_waypoint;
pub use crate::sim::set_primary_computer_waypoint_c;
pub use crate::sim::spawn_empty_grid;
pub use crate::sim::split_grid_if_necessary;
pub use crate::sim::update_grid_acceleration;
pub use crate::sim::update_grid_physical_props_by_id;
pub use crate::sim::update_single_grid_acceleration;
pub use crate::sim::update_world;
