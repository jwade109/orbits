pub mod app;
pub mod camera;
pub mod chat;
pub mod command_prompt;
pub mod components;
pub mod draw;
pub mod input_state;
pub mod multiplayer;
pub mod persistence;
pub mod result;
pub mod ring_particle;
pub mod sounds;
pub mod systems;
pub mod tests;
pub mod ui;
pub mod utils;
pub mod vehicle;
pub mod wall_timer;
pub mod world;
pub mod world_builder;

/// A namespace of functions which are potentially expensive,
/// but nonetheless offer a readonly way to retrieve some information
/// of interest. Some of these are acceptable to run during sims;
/// others should only be used to verify correctness in tests.
pub mod query {
    pub use crate::systems::find::blueprint_by_name;
    pub use crate::systems::find::closest_grid;
    pub use crate::systems::find::grid_by_name;
    pub use crate::systems::find::grid_pose;
    pub use crate::systems::find::primary_computer_id;
    pub use crate::systems::find::proto_by_name;
    pub use crate::systems::find::sum_part_masses;
    pub use crate::systems::find::sum_part_masses_w;
}

/// A namespace of functions which perform operations in the simulation world
/// and maintain certain invariants. Using these is safer than performing
/// direct manipulation of world entities, though potentially costlier
/// since using several in succession might perform duplicate lookups.
pub mod ops {
    pub use crate::systems::insert_part_c;
    pub use crate::systems::set_primary_computer_state_c;
    pub use crate::systems::set_primary_computer_waypoint_c;
    pub use crate::systems::spawn_empty_grid;
    pub use crate::systems::spawn_empty_grid_c;
    pub use crate::systems::world::insert_part;
    pub use crate::systems::world::set_grid_pose;
    pub use crate::systems::world::set_grid_vel;
    pub use crate::systems::world::set_primary_computer_state;
    pub use crate::systems::world::set_primary_computer_waypoint;
    pub use crate::vehicle::grid::detach_part_from_parent;
    pub use crate::vehicle::grid::duplicate_part_to_new_grid;
    pub use crate::vehicle::grid::remove_part_without_integrity_check;
    pub use crate::vehicle::grid::split_grid_if_necessary_todo_implement_me;
    pub use crate::world::update_computers;
    pub use crate::world::update_trackers;
    pub use crate::world::update_world;
}
