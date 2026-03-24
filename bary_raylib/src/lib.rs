pub mod app;
pub mod assets;
pub mod camera;
pub mod client;
pub mod cmd;
pub mod components;
pub mod constants;
pub mod input_state;
pub mod multiplayer;
pub mod persistence;
pub mod render;
pub mod result;
pub mod sim;
pub mod sounds;
pub mod tests;
pub mod ui;
pub mod utils;
pub mod wall_timer;
pub mod world_builder;

/// A namespace of functions which are potentially expensive,
/// but nonetheless offer a readonly way to retrieve some information
/// of interest. Some of these are acceptable to run during sims;
/// others should only be used to verify correctness in tests.
pub mod query {
    pub use crate::sim::systems::find::blueprint_by_name;
    pub use crate::sim::systems::find::closest_grid;
    pub use crate::sim::systems::find::grid_by_name;
    pub use crate::sim::systems::find::grid_origin;
    pub use crate::sim::systems::find::primary_computer_id;
    pub use crate::sim::systems::find::proto_by_name;
    pub use crate::sim::systems::find::sum_part_masses;
    pub use crate::sim::systems::find::sum_part_masses_w;
    pub use crate::sim::systems::get_grid_physical_props;
    pub use crate::sim::systems::get_grid_physical_props_by_id;
    pub use crate::sim::systems::get_sum_linear_forces;
}

/// A namespace of functions which perform operations in the simulation world
/// and maintain certain invariants. Using these is safer than performing
/// direct manipulation of world entities, though potentially costlier
/// since using several in succession might perform duplicate lookups.
pub mod ops {
    pub use crate::sim::systems::despawn_all_vehicles;
    pub use crate::sim::systems::despawn_grid;
    pub use crate::sim::systems::insert_part;
    pub use crate::sim::systems::insert_part_c;
    pub use crate::sim::systems::set_primary_computer_state_c;
    pub use crate::sim::systems::set_primary_computer_waypoint_c;
    pub use crate::sim::systems::spawn_empty_grid;
    pub use crate::sim::systems::spawn_empty_grid_c;
    pub use crate::sim::systems::update_grid_acceleration;
    pub use crate::sim::systems::update_grid_physical_props;
    pub use crate::sim::systems::update_grid_physical_props_by_id;
    pub use crate::sim::systems::world::set_all_thrusters;
    pub use crate::sim::systems::world::set_grid_pose;
    pub use crate::sim::systems::world::set_grid_vel;
    pub use crate::sim::systems::world::set_primary_computer_state;
    pub use crate::sim::systems::world::set_primary_computer_waypoint;
    pub use crate::sim::systems::world::set_thruster_state;
    pub use crate::sim::vehicle::destroy_part_without_integrity_check;
    pub use crate::sim::vehicle::detach_part_from_parent;
    pub use crate::sim::vehicle::duplicate_part_to_new_grid;
    pub use crate::sim::vehicle::rebuild_index_from_island;
    pub use crate::sim::vehicle::split_grid_if_necessary;
    pub use crate::sim::vehicle::update_computers;
    pub use crate::sim::world::update_trackers;
    pub use crate::sim::world::update_world;
}
