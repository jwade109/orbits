use std::collections::BTreeSet;

use crate::components::*;
use crate::result::*;
use crate::vehicle::*;
use crate::world::*;
use bary_core::prelude::*;
use log::{debug, info};
use std::time::Duration;

pub const TICKS_PER_SECOND: u64 = 50;

pub fn apparent_elapsed_time(world: &World) -> Duration {
    Duration::from_millis(1000 / TICKS_PER_SECOND * world.ticks)
}

pub fn spawn_grid_from_blueprint(
    counter: &mut EntitySpawner,
    prototypes: &Components<PartPrototype>,
    grids: &mut Components<VehicleGrid>,
    parts: &mut Components<Part>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    lights: &mut Components<Light>,
    name: impl Into<String>,
    bp: &Blueprint,
) -> BaryResult<Ent> {
    let s = name.into();
    info!("Spawning grid with name \"{}\" from blueprint", s);
    let grid = VehicleGrid::with_name(s);
    let grid_id = counter.spawn();
    grids.spawn(grid_id, grid.clone());
    for (_id, proto) in bp.parts() {
        insert_part_c(
            grid_id, counter, grids, prototypes, parts, thrusters, computers, lights, proto,
        )?;
    }
    Ok(grid_id)
}

pub fn body_frame_wrench(
    thrust: f32,
    center_of_thrust: Vec2,
    rotation: Rotation,
    com: Vec2,
) -> Isometry2d {
    let u = rotation.to_dir();
    let lever_arm = center_of_thrust - com;
    let thrust = thrust * u.as_vec2();
    let torque = cross2d(lever_arm, thrust);
    Isometry2d::new(thrust, torque as f32)
}

pub fn update_grid_acceleration(
    dirty_set: BTreeSet<Ent>,
    grids: &mut Components<VehicleGrid>,
    thrusters: &Components<Thruster>,
    parts: &Components<Part>,
) {
    for grid_id in dirty_set {
        let Ok(grid) = grids.try_get_mut(grid_id) else {
            continue;
        };
        grid.body_frame_forces = Isometry2d::ZERO;
        for thruster_id in &grid.thrusters {
            let Ok(thruster) = thrusters.try_get(*thruster_id) else {
                continue;
            };

            if !thruster.is_on {
                continue;
            }

            let Ok(part) = parts.try_get(*thruster_id) else {
                continue;
            };

            let center_of_thrust = part.placement.center_isometry().translation;
            let rotation = part.placement.rot();
            let wrench = body_frame_wrench(thruster.thrust, center_of_thrust, rotation, Vec2::ZERO);
            grid.body_frame_forces.translation += wrench.translation;
            grid.body_frame_forces.rotation += wrench.rotation;
        }
    }
}

/// Sets the waypoint field of the primary computer,
/// if the provided grid has one. If it does, the ID of the primary
/// computer will be returned.
/// TODO(testing) test this.
pub fn set_primary_computer_waypoint_c(
    grid_id: Ent,
    waypoint: Isometry2d,
    grids: &Components<VehicleGrid>,
    computers: &mut Components<Computer>,
) -> BaryResult<Ent> {
    let primary_cpu_id = find::primary_computer_id(grid_id, grids)?;
    let computer = computers.try_get_mut(primary_cpu_id)?;
    computer.pose = waypoint;
    Ok(primary_cpu_id)
}

/// Turns the primary computer of the given grid on or off,
/// returning the entity ID of the computer if it was found.
/// TODO(testing) test this.
pub fn set_primary_computer_state_c(
    grid_id: Ent,
    new_state: bool,
    grids: &Components<VehicleGrid>,
    computers: &mut Components<Computer>,
) -> BaryResult<Ent> {
    let primary_cpu_id = find::primary_computer_id(grid_id, grids)?;
    let computer = computers.try_get_mut(primary_cpu_id)?;
    computer.on = new_state;
    Ok(primary_cpu_id)
}

/// Spawns an empty grid with the given name.
/// Exclusive version of [`super::spawn_empty_grid`].
pub fn spawn_empty_grid(world: &mut World, name: impl Into<String>) -> Ent {
    spawn_empty_grid_c(&mut world.spawner, &mut world.grids, name)
}

pub mod world {
    use crate::ring_particle::PingParticle;

    use super::*;

    pub fn toggle_tracking(world: &mut World, grid_id: Ent) -> BaryResult<bool> {
        let tracking = if world.tracking.contains_key(&grid_id) {
            world.tracking.despawn(grid_id)?;
            info!("Removed tracking for grid {}", grid_id);
            false
        } else {
            world.tracking.spawn(grid_id, Tracker::default());
            info!("Enabled tracking for grid {}", grid_id);
            true
        };
        Ok(tracking)
    }

    /// Sets the waypoint field of the primary computer,
    /// if the provided grid has one. If it does, the ID of the primary
    /// computer will be returned.
    /// TODO(testing) test this.
    pub fn set_primary_computer_waypoint(
        grid_id: Ent,
        waypoint: Isometry2d,
        world: &mut World,
    ) -> BaryResult<Ent> {
        super::set_primary_computer_waypoint_c(grid_id, waypoint, &world.grids, &mut world.computers)
    }

    /// Turns the primary computer of the given grid on or off,
    /// returning the entity ID of the computer if it was found.
    pub fn set_primary_computer_state(
        grid_id: Ent,
        new_state: bool,
        world: &mut World,
    ) -> BaryResult<Ent> {
        super::set_primary_computer_state_c(grid_id, new_state, &world.grids, &mut world.computers)
    }

    pub fn update_grid_acceleration(dirty_set: BTreeSet<Ent>, world: &mut World) {
        super::update_grid_acceleration(
            dirty_set,
            &mut world.grids,
            &world.thrusters,
            &world.parts,
        );
    }

    /// Spawns a grid according to the given blueprint.
    /// Exclusive version of [`super::spawn_grid_from_blueprint`].
    pub fn spawn_grid_from_blueprint(
        world: &mut World,
        name: impl Into<String>,
        bp: &Blueprint,
    ) -> BaryResult<Ent> {
        super::spawn_grid_from_blueprint(
            &mut world.spawner,
            &mut world.prototypes,
            &mut world.grids,
            &mut world.parts,
            &mut world.thrusters,
            &mut world.computers,
            &mut world.lights,
            name,
            bp,
        )
    }

    pub fn set_grid_pose(world: &mut World, grid_id: Ent, iso: Isometry2d) -> BaryResult<()> {
        info!("Setting isometry of grid {} to {:?}", grid_id, iso);
        let grid = world.grids.try_get_mut(grid_id)?;
        grid.pose = iso;
        Ok(())
    }

    pub fn set_grid_vel(world: &mut World, grid_id: Ent, vel: Isometry2d) -> BaryResult<()> {
        info!("Setting velocity of grid {} to {:?}", grid_id, vel);
        let grid = world.grids.try_get_mut(grid_id)?;
        grid.velocity = vel;
        Ok(())
    }

    /// Spawns a new grid according to a named blueprint.
    pub fn spawn_grid_by_name(world: &mut World, name: &str) -> BaryResult<Ent> {
        let bp = find::blueprint_by_name(&world.blueprints, name)
            .ok_or(BaryError::BadBlueprint)?
            .clone();
        spawn_grid_from_blueprint(world, name, &bp)
    }

    /// Inserts a part into an existing grid.
    /// Exclusive version of [`super::insert_part_c`].
    pub fn insert_part(
        grid_id: Ent,
        world: &mut World,
        instance: &PartInstance,
    ) -> BaryResult<Ent> {
        super::insert_part_c(
            grid_id,
            &mut world.spawner,
            &mut world.grids,
            &mut world.prototypes,
            &mut world.parts,
            &mut world.thrusters,
            &mut world.computers,
            &mut world.lights,
            instance,
        )
    }

    /// Sets the state of a given thruster.
    /// Does not modify the corresponding grid's acceleration.
    /// TODO(cleanup) this doesn't really need to be a function.
    /// Exclusive version of [`super::set_thruster_state`].
    pub fn set_thruster_state(
        thruster_id: Ent,
        world: &mut World,
        new_state: bool,
    ) -> BaryResult<()> {
        super::set_thruster_state(thruster_id, &mut world.thrusters, new_state)
    }

    pub fn ping(world: &mut World, pos: Vec2) {
        let part = PingParticle::new(pos);
        world.particles.push(part);
    }

    pub fn get_blueprint(world: &World, grid_id: Ent) -> BaryResult<Blueprint> {
        super::get_blueprint(&world.grids, &world.parts, &world.prototypes, grid_id)
    }
}

/// Spawns an empty vehicle grid.
pub fn spawn_empty_grid_c(
    spawner: &mut EntitySpawner,
    grids: &mut Components<VehicleGrid>,
    name: impl Into<String>,
) -> Ent {
    let name = name.into();
    debug!("Spawning empty grid with name {}", name);
    let grid = VehicleGrid::with_name(name);
    let id = spawner.spawn();
    grids.spawn(id, grid);
    id
}

pub fn insert_part_c(
    grid_id: Ent,
    counter: &mut EntitySpawner,
    grids: &mut Components<VehicleGrid>,
    prototypes: &Components<PartPrototype>,
    parts: &mut Components<Part>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    lights: &mut Components<Light>,
    instance: &PartInstance,
) -> BaryResult<Ent> {
    debug!(
        "Inserting part \"{}\" into grid \"{}\" in layer {:?}",
        instance.name, grid_id, instance.layer
    );
    let grid = grids.try_get_mut(grid_id)?;
    let proto_id = find::proto_by_name(prototypes, &instance.name).ok_or(BaryError::BadPartName)?;
    let proto = prototypes.try_get(proto_id)?;

    grid.parts_mass += proto.mass;

    let part = Part {
        placement: instance.placement,
        layer: instance.layer(),
        prototype: proto_id,
        grid_id,
        classification: proto.classification(),
    };

    let part_id = counter.spawn();

    grid.parts.insert(part_id);
    parts.spawn(part_id, part);

    grid.mark_occupied(instance.placement, instance.layer(), part_id);

    if let Some(data) = &proto.thruster_data {
        let thruster = Thruster {
            is_on: false,
            is_rcs: data.is_rcs,
            // TODO(gross)
            thrust: data.thrust as f32,
            prototype: proto_id,
            grid_id,
            last_controlled_by: None,
        };
        thrusters.spawn(part_id, thruster);
        grid.thrusters.insert(part_id);
    }
    if let Some(_data) = &proto.computer_data {
        let cpu = Computer::new(grid_id, proto_id);
        computers.spawn(part_id, cpu);
        grid.computers.insert(part_id);
    }
    if let Some(data) = &proto.thruster_data {
        if data.is_rcs {
            let light_idx = lights.len();
            let light = Light::new(light_idx as u32);
            lights.spawn(part_id, light);
            grid.lights.insert(part_id);
        }
    }

    Ok(part_id)
}

pub fn despawn_grid(
    grid_id: Ent,
    grids: &mut Components<VehicleGrid>,
    parts: &mut Components<Part>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    lights: &mut Components<Light>,
) -> BaryResult<()> {
    let grid = grids.despawn(grid_id)?;
    for id in grid.parts {
        parts.despawn(id)?;
    }
    for id in grid.thrusters {
        thrusters.despawn(id)?;
    }
    for id in grid.computers {
        computers.despawn(id)?;
    }
    for id in grid.lights {
        lights.despawn(id)?;
    }
    Ok(())
}

pub mod find {
    use log::error;

    use super::*;

    /// Produces the entity ID corresponding to a grid's primary CPU,
    /// which by convention is just the first element in the computer index.
    pub fn primary_computer_id(grid_id: Ent, grids: &Components<VehicleGrid>) -> BaryResult<Ent> {
        let grid = grids.try_get(grid_id)?;
        Ok(*grid.computers.first().ok_or(BaryError::NoPrimaryComputer)?)
    }

    pub fn sum_part_masses(
        grids: &Components<VehicleGrid>,
        parts: &Components<Part>,
        prototypes: &Components<PartPrototype>,
        grid_id: Ent,
    ) -> BaryResult<Mass> {
        let grid = grids.try_get(grid_id)?;
        let mut sum = Mass::ZERO;
        for part_id in &grid.parts {
            let part = parts.try_get(*part_id)?;
            let proto = prototypes.try_get(part.prototype)?;
            sum += proto.mass;
        }
        Ok(sum)
    }

    pub fn sum_part_masses_w(world: &World, grid_id: Ent) -> BaryResult<Mass> {
        sum_part_masses(&world.grids, &world.parts, &world.prototypes, grid_id)
    }

    pub fn blueprint_by_name<'a>(
        blueprints: &'a Components<NamedBlueprint>,
        name: &str,
    ) -> Option<&'a Blueprint> {
        let result = blueprints
            .values()
            .find(|(n, _bp)| n == name)
            .map(|(_, bp)| bp);

        if result.is_none() {
            error!("Failed to get blueprint with name {}", name);
        }

        result
    }

    /// Produces whatever prototype has the given name, if any.
    pub fn proto_by_name(prototypes: &Components<PartPrototype>, name: &str) -> Option<Ent> {
        prototypes
            .iter()
            .find(|(_, proto)| proto.part_name() == name)
            .map(|e| *e.0)
    }

    pub fn grid_pose(grids: &Components<VehicleGrid>, grid_id: Ent) -> Option<Isometry2d> {
        let grid = grids.try_get(grid_id).ok()?;
        Some(grid.pose)
    }

    /// Returns the ID of the first grid in the components list with
    /// the given name.
    ///
    /// Buyer beware: grid names are not unique! This
    /// only promises to return any grid with the given name, if one exists.
    pub fn grid_by_name(grids: &Components<VehicleGrid>, name: &str) -> Option<Ent> {
        grids
            .iter()
            .find_map(|(id, grid)| (grid.name == name).then(|| *id))
    }

    pub fn closest_grid(
        grids: &Components<VehicleGrid>,
        test_pos: Vec2,
        dist_limit: impl Into<Option<f32>>,
    ) -> Option<(Ent, Vec2)> {
        let mut best: Option<(Ent, Vec2, f32)> = None;
        let dist_limit = dist_limit.into().unwrap_or(std::f32::INFINITY);
        for (e, grid) in grids.iter() {
            let in_frame = express_in_frame(grid.pose, test_pos);
            let dist = in_frame.length_squared();
            if dist > dist_limit {
                continue;
            }
            if let Some(best) = &mut best {
                if dist < best.2 {
                    best.0 = *e;
                    best.1 = in_frame;
                    best.2 = dist;
                }
            } else {
                best = Some((*e, in_frame, dist));
            }
        }
        best.map(|x| (x.0, x.1))
    }
}

pub fn get_blueprint(
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    prototypes: &Components<PartPrototype>,
    grid_id: Ent,
) -> BaryResult<Blueprint> {
    let grid = grids.try_get(grid_id)?;
    let mut bp = Blueprint::new();
    for part_id in &grid.parts {
        let part = parts.try_get(*part_id)?;
        let proto = prototypes.try_get(part.prototype)?;
        bp.add_part(proto.name.to_string(), part.placement, part.layer);
    }
    Ok(bp)
}

pub fn get_sum_linear_forces(
    grid_id: Ent,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    thrusters: &Components<Thruster>,
) -> BaryResult<Vec2> {
    let grid = grids.try_get(grid_id)?;
    let mut sum = Vec2::ZERO;
    for part_id in &grid.thrusters {
        let thruster = thrusters.try_get(*part_id)?;
        let part = parts.try_get(*part_id)?;
        let thrust = rotate(Vec2::X, part.placement.rot().to_angle() as f32) * thruster.thrust;
        sum += thrust;
    }
    Ok(sum)
}

pub fn get_parts_center_of_mass(grid_id: Ent, world: &World) -> BaryResult<Vec2> {
    let grid = world.grids.try_get(grid_id)?;
    let mut total_mass = Mass::ZERO;
    for part_id in &grid.parts {
        let part = world.parts.try_get(*part_id)?;
        let proto = world.prototypes.try_get(part.prototype)?;
        total_mass += proto.mass;
    }
    let mut com = Vec2::ZERO;
    for part_id in &grid.parts {
        let part = world.parts.try_get(*part_id)?;
        let proto = world.prototypes.try_get(part.prototype)?;
        let center = part.placement.center_isometry();
        let mass_portion = proto.mass.to_kg_f64() / total_mass.to_kg_f64();
        com += center.translation * mass_portion as f32;
    }
    Ok(com)
}

fn set_thruster_state(
    thruster_id: Ent,
    thrusters: &mut Components<Thruster>,
    new_state: bool,
) -> BaryResult<()> {
    let thruster = thrusters.try_get_mut(thruster_id)?;
    thruster.is_on = new_state;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tests::assert_world_is_consistent, world_builder::WorldBuilder};

    #[test]
    fn part_prototypes() {
        let world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let mut iter = world.prototypes.iter();

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(0));
        assert_eq!(proto.part_name(), "angled-frame");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(1));
        assert_eq!(proto.part_name(), "antenna");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(2));
        assert_eq!(proto.part_name(), "battery");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(3));
        assert_eq!(proto.part_name(), "cargo");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(4));
        assert_eq!(proto.part_name(), "chemical-plant");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(5));
        assert_eq!(proto.part_name(), "container");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(6));
        assert_eq!(proto.part_name(), "cpu");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(7));
        assert_eq!(proto.part_name(), "debug-item-source");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(8));
        assert_eq!(proto.part_name(), "docking-port");

        assert_world_is_consistent(&world);
    }

    #[test]
    fn vehicle_spawning_and_despawning() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let name = "pollux";

        // get the blueprint for the pollux
        let bp = find::blueprint_by_name(&world.blueprints, name).expect("Expected a blueprint");

        // spawn that vehicle using its blueprint
        let grid_id = spawn_grid_from_blueprint(
            &mut world.spawner,
            &world.prototypes,
            &mut world.grids,
            &mut world.parts,
            &mut world.thrusters,
            &mut world.computers,
            &mut world.lights,
            name.to_string(),
            &bp,
        )
        .expect("Expected the grid ID");

        let expected_grid_id = Ent(34);

        // this entity should be the same every time
        assert_eq!(grid_id, expected_grid_id);

        // the mass should already be computed
        let grid = world.grids.get(expected_grid_id).unwrap();
        assert_eq!(grid.parts_mass, Mass::grams(35134000));

        assert_eq!(world.grids.len(), 1);
        assert_eq!(world.parts.len(), 98);
        assert_eq!(world.thrusters.len(), 18);
        assert_eq!(world.computers.len(), 1);
        assert_eq!(world.lights.len(), 12);

        // get the computer entity
        let (id, cpu) = world.computers.iter().next().unwrap();

        // these entities should be the same every time
        assert_eq!(*id, Ent(58));
        assert_eq!(cpu.grid_id, expected_grid_id);
        assert_eq!(cpu.prototype, Ent(6));

        // get the prototype definition for the computer
        let proto = world.prototypes.get(cpu.prototype).unwrap();

        // it should be the "cpu" part
        assert_eq!(proto.part_name(), "cpu");

        // despawning should work, of course
        let result = despawn_grid(
            grid_id,
            &mut world.grids,
            &mut world.parts,
            &mut world.thrusters,
            &mut world.computers,
            &mut world.lights,
        );
        assert_eq!(result, Ok(()));

        // now the world should be empty
        assert_eq!(world.grids.len(), 0);
        assert_eq!(world.parts.len(), 0);
        assert_eq!(world.thrusters.len(), 0);
        assert_eq!(world.computers.len(), 0);
        assert_eq!(world.lights.len(), 0);

        // doing this again should return an error
        let result = despawn_grid(
            grid_id,
            &mut world.grids,
            &mut world.parts,
            &mut world.thrusters,
            &mut world.computers,
            &mut world.lights,
        );

        assert_eq!(result, Err(BaryError::EntityNotFound(grid_id)));

        assert_world_is_consistent(&world);
    }

    #[test]
    fn nearest_grid() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        assert!(find::closest_grid(&world.grids, Vec2::new(100.0, 200.0), None).is_none());

        let id = world::spawn_grid_by_name(&mut world, "remora").unwrap();
        assert_eq!(id, Ent(34));

        let grid = world.grids.try_get_mut(id).unwrap();
        grid.pose.translation = (40.0, 156.0).into();

        for _ in 0..100 {
            update_world(&mut world);
            let test_pos = Vec2::new(100.0, 200.0);
            let e = find::closest_grid(&world.grids, test_pos, None);
            assert_eq!(e, Some((Ent(34), Vec2::new(60.0, 44.0))));
        }

        assert_world_is_consistent(&world);
    }

    #[test]
    fn insert_parts() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .build();

        let part_name = "motor";

        let proto_id = find::proto_by_name(&world.prototypes, part_name).unwrap();

        let proto = world.prototypes.try_get(proto_id).unwrap();
        let dims = proto.dims;

        assert_eq!(proto_id, Ent(16));

        let grid_id = world::spawn_grid_by_name(&mut world, "pollux").unwrap();

        assert_eq!(world.parts.len(), 98);
        assert_eq!(world.thrusters.len(), 18);

        assert_eq!(grid_id, Ent(31));

        let instance = PartInstance::new(
            part_name,
            PartLayer::Internal,
            GridPlacement::new((2, 3), Rotation::East, dims),
        );

        let id = world::insert_part(grid_id, &mut world, &instance).unwrap();

        assert_eq!(id, Ent(130));

        let part = world.parts.get(id).unwrap();

        assert_eq!(part.grid_id, grid_id);
        assert_eq!(part.prototype, proto_id);
        assert_eq!(
            part.placement,
            // TODO allow insertion at a given placement
            GridPlacement::new((2, 3), Rotation::East, (6, 3))
        );

        assert_eq!(world.parts.len(), 99);
        assert_eq!(world.thrusters.len(), 19);

        assert_world_is_consistent(&world);
    }

    #[test]
    fn parts_mass() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let id = world::spawn_grid_by_name(&mut world, "pollux").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(35134000));

        let id = world::spawn_grid_by_name(&mut world, "bellerophon").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(178051000));

        let id = world::spawn_grid_by_name(&mut world, "remora").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(12339000));

        let id = world::spawn_grid_by_name(&mut world, "spacestation").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(145638000));

        assert_world_is_consistent(&world);
    }

    #[test]
    fn calculate_blueprints() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let expected = find::blueprint_by_name(&world.blueprints, "pollux")
            .unwrap()
            .clone();

        let id = world::spawn_grid_by_name(&mut world, "pollux").unwrap();

        let actual = get_blueprint(&world.grids, &world.parts, &world.prototypes, id).unwrap();

        assert_eq!(actual.part_count(), expected.part_count());

        for (a, b) in actual.parts().zip(expected.parts()) {
            assert_eq!(a.0, b.0);
            assert_eq!(a.1, b.1);
        }

        // TODO this test needs to be revived once pipes are added
        // to the new ECS
        // assert_eq!(actual.pipe_count(), expected.pipe_count());

        // failing because pipes aren't implemented.
        // assert_eq!(actual, expected);

        assert_world_is_consistent(&world);
    }

    #[test]
    fn bad_part_insertion() {
        let mut world = WorldBuilder::new().test_assets().build();
        let id = spawn_empty_grid(&mut world, "whatever");

        let instance = PartInstance::new(
            "dingus",
            PartLayer::Internal,
            GridPlacement::new((0, 0), Rotation::East, (3, 3)),
        );

        let result = world::insert_part(id, &mut world, &instance);

        assert_eq!(result, Err(BaryError::BadPartName));

        let instance = PartInstance::new(
            "cargo",
            PartLayer::Internal,
            GridPlacement::new((0, 0), Rotation::East, (3, 3)),
        );

        let result = world::insert_part(Ent(103), &mut world, &instance);

        assert_eq!(result, Err(BaryError::EntityNotFound(Ent(103))));

        assert_world_is_consistent(&world);
    }

    #[test]
    fn set_thruster_state() {
        let mut world = WorldBuilder::new().test_assets().build();

        let grid_id = spawn_empty_grid(&mut world, "whatever");

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts.len(), 0);
        assert_eq!(grid.parts_mass, Mass::ZERO);

        assert_eq!(grid_id, Ent(30));

        let instance_a = PartInstance::new(
            "motor",
            PartLayer::Internal,
            GridPlacement::new((0, 0), Rotation::East, (6, 3)),
        );

        let instance_b = PartInstance::new(
            "small-motor",
            PartLayer::Internal,
            GridPlacement::new((0, 0), Rotation::North, (4, 2)),
        );

        let a_id = world::insert_part(grid_id, &mut world, &instance_a).unwrap();
        let b_id = world::insert_part(grid_id, &mut world, &instance_b).unwrap();

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts.len(), 2);
        assert_eq!(grid.parts, [a_id, b_id].into());
        assert_eq!(grid.thrusters, [a_id, b_id].into());
        assert_eq!(grid.parts_mass, Mass::grams(870000));

        assert_eq!(a_id, Ent(31));
        assert_eq!(b_id, Ent(32));

        let r1 = world::set_thruster_state(a_id, &mut world, true);
        let r2 = world::set_thruster_state(b_id, &mut world, true);

        world::update_grid_acceleration([grid_id].into(), &mut world);

        assert_eq!(r1, Ok(()));
        assert_eq!(r2, Ok(()));

        let sum =
            get_sum_linear_forces(grid_id, &world.grids, &world.parts, &world.thrusters).unwrap();

        assert_eq!(sum.x, 400000.0);
        assert_eq!(sum.y, 320000.0);

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(
            grid.body_frame_forces.translation,
            Vec2::new(400000.0, 320000.0)
        );

        let r1 = world::set_thruster_state(a_id, &mut world, false);
        let r2 = world::set_thruster_state(b_id, &mut world, true);

        world::update_grid_acceleration([grid_id].into(), &mut world);

        assert_eq!(r1, Ok(()));
        assert_eq!(r2, Ok(()));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.body_frame_forces.translation, Vec2::new(0.0, 320000.0));

        let r1 = world::set_thruster_state(a_id, &mut world, false);
        let r2 = world::set_thruster_state(b_id, &mut world, false);

        world::update_grid_acceleration([grid_id].into(), &mut world);

        assert_eq!(r1, Ok(()));
        assert_eq!(r2, Ok(()));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(
            grid.body_frame_forces.translation,
            IVec2::new(0, 0).as_vec2()
        );
    }

    #[test]
    fn parts_center_of_mass() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .build();

        let id = world::spawn_grid_by_name(&mut world, "pollux").unwrap();
        let com = get_parts_center_of_mass(id, &mut world).unwrap();

        // TODO is this right? possibly. seems close enough
        assert_eq!(com, Vec2::new(0.001067417, 0.022271877));

        let cargo_id = find::proto_by_name(&world.prototypes, "cargo").unwrap();
        let cargo_proto = world.prototypes.try_get(cargo_id).unwrap();

        assert_eq!(cargo_proto.dims, (6, 6).into());
        assert_eq!(cargo_proto.dims_meters(), (1.5, 1.5).into());

        let instance = PartInstance::from_prototype(cargo_proto, (0, 0).into(), Rotation::East);

        let grid_id = spawn_empty_grid(&mut world, "whatever");
        _ = world::insert_part(grid_id, &mut world, &instance);

        let com = get_parts_center_of_mass(grid_id, &mut world).unwrap();

        assert_eq!(com, Vec2::splat(0.75));
    }

    #[test]
    fn pure_linear_acceleration() {
        let mut world = WorldBuilder::new().test_assets().build();

        // modifying the prototype for motor so it has easy quantities
        let proto_id = find::proto_by_name(&world.prototypes, "small-motor").unwrap();
        let proto = world.prototypes.try_get_mut(proto_id).unwrap();

        proto.mass = Mass::kilograms(1000);
        if let Some(t) = &mut proto.thruster_data {
            // 3500 newtons
            t.thrust = 3500.0;
        }

        let dims = proto.dims;

        let grid_id = spawn_empty_grid(&mut world, "testbed");

        let instance = PartInstance {
            name: "small-motor".to_string(),
            layer: PartLayer::Internal,
            placement: GridPlacement::new((0, -1), Rotation::East, dims),
        };

        let thruster_id = world::insert_part(grid_id, &mut world, &instance).unwrap();

        let center_isometry = instance.placement.center_isometry();

        let com = get_parts_center_of_mass(grid_id, &world);
        assert_eq!(com, Ok(Vec2::X * center_isometry.translation.x));

        // obviously, turn the main thruster on
        let r = world::set_thruster_state(thruster_id, &mut world, true);
        assert_eq!(r, Ok(()));

        world::update_grid_acceleration([grid_id].into(), &mut world);

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.body_frame_forces.translation, Vec2::new(3500.0, 0.0));
        assert_eq!(grid.body_frame_forces.rotation, 0.0);
        assert_eq!(grid.parts_mass, Mass::kilograms(1000));

        // body frame acceleration should be 3.5 m/s^2
        assert_eq!(grid.linear_acceleration(), Vec2::new(3.5, 0.0));
        assert_eq!(grid.angular_acceleration(), 0.0);

        // run the simulation for 2 seconds at 50 Hz
        for _ in 0..100 {
            update_world(&mut world);
        }

        let iso = world.grids.try_get(grid_id).unwrap().pose;

        // this is an approximation of the following
        // continuous time kinematic equation:
        // d = 1/2 at^2  --> 0.5 * 3.5 * 2^2 = 7
        assert_eq!(iso.translation, Vec2::new(6.9299994, 0.0));
        assert_eq!(iso.rotation, 0.0);
    }

    #[test]
    fn under_linear_and_angular_acceleration() {
        let mut world = World::empty();

        let part_name = "test-motor";

        let thruster_data = ThrusterModel {
            model: "test-motor-model".to_string(),
            thrust: 8000.0,
            exhaust_velocity: 6000.0,
            is_rcs: false,
            throttle_rate: 0.0,
            primary_color: [0.0, 0.0, 0.0, 0.0],
            secondary_color: [0.0, 0.0, 0.0, 0.0],
            plume_length: 1.0,
            plume_angle: 0.1,
            minimum_throttle: 0.0,
            particle_scale: 1.0,
        };

        let proto = PartPrototype {
            name: part_name.to_string(),
            mass: Mass::kilograms(1000),
            dims: UVec2::new(4, 2),
            layer: PartLayer::Internal,
            excavator_data: None,
            computer_data: None,
            inventory_data: None,
            thruster_data: Some(thruster_data),
            machine_data: None,
            docking_port_data: None,
        };

        let dims = proto.dims;

        let proto_id = world.spawner.spawn();
        world.prototypes.spawn(proto_id, proto);

        let placement = GridPlacement::new((0, 0), Rotation::East, dims);

        let grid_id = spawn_empty_grid(&mut world, "testbed");

        let instance = PartInstance {
            name: part_name.to_string(),
            layer: PartLayer::Internal,
            placement,
        };

        use find::grid_pose;

        let thruster_id = world::insert_part(grid_id, &mut world, &instance).unwrap();

        _ = world::set_thruster_state(thruster_id, &mut world, true);
        world::update_grid_acceleration([grid_id].into(), &mut world);

        assert_eq!(grid_pose(&world.grids, grid_id), Some(Isometry2d::ZERO));

        let expected_poses = [
            (0.0, 0.0, 0.0),
            (0.0032, 0.0, -0.000008),
            (0.0095999995, 0.0, -0.000024),
            (0.019199999, -0.000000025599999, -0.000048),
            (0.031999998, -0.000000128, -0.00008),
            (0.047999997, -0.00000038399997, -0.00012),
            (0.06719999, -0.00000089599996, -0.000168),
            (0.08959999, -0.0000017919999, -0.00022399999),
            (0.11519998, -0.0000032255998, -0.00028799998),
            (0.14399998, -0.0000053759995, -0.00035999998),
            (0.17599997, -0.000008448, -0.00043999997),
            (0.21119997, -0.000012671999, -0.00052799995),
            (0.24959996, -0.000018304, -0.0006239999),
            (0.29119995, -0.000025625599, -0.0007279999),
            (0.33599994, -0.000034943998, -0.0008399999),
            (0.38399994, -0.000046591995, -0.00095999986),
            (0.43519992, -0.000060927992, -0.0010879998),
            (0.4895999, -0.00007833599, -0.0012239998),
            (0.5471999, -0.00009922558, -0.0013679997),
            (0.60799986, -0.00012403197, -0.0015199997),
            (0.6719998, -0.00015321597, -0.0016799996),
            (0.73919976, -0.00018726396, -0.0018479996),
            (0.80959976, -0.00022668793, -0.0020239996),
            (0.8831997, -0.0002720255, -0.0022079996),
            (0.9599996, -0.00032383986, -0.0023999996),
            (1.0399996, -0.00038271982, -0.0025999995),
            (1.1231996, -0.0004492798, -0.0028079995),
            (1.2095995, -0.00052415975, -0.0030239995),
            (1.2991995, -0.0006080253, -0.0032479996),
            (1.3919994, -0.0007015676, -0.0034799995),
            (1.4879992, -0.0008055035, -0.0037199995),
            (1.5871991, -0.0009205754, -0.0039679995),
            (1.6895989, -0.0010475513, -0.0042239996),
            (1.7951987, -0.0011872246, -0.0044879997),
            (1.9039985, -0.0013404149, -0.00476),
            (2.0159984, -0.0015079665, -0.0050399997),
            (2.1311982, -0.0016907502, -0.0053279996),
            (2.2495978, -0.0018896619, -0.0056239995),
            (2.3711975, -0.002105623, -0.0059279995),
            (2.495997, -0.0023395808, -0.0062399996),
            (2.6239965, -0.0025925082, -0.0065599997),
            (2.7551959, -0.0028654034, -0.006888),
            (2.8895953, -0.0031592904, -0.007224),
            (3.0271945, -0.0034752188, -0.007568),
            (3.1679938, -0.0038142637, -0.00792),
            (3.311993, -0.004177526, -0.00828),
            (3.459192, -0.004566132, -0.008648),
            (3.609591, -0.0049812337, -0.009024),
            (3.7631898, -0.0054240087, -0.009408),
            (3.9199884, -0.0058956603, -0.0098),
            (4.079987, -0.006397417, -0.0102),
            (4.2431855, -0.006930533, -0.010608001),
            (4.4095836, -0.0074962885, -0.011024),
            (4.5791817, -0.008095989, -0.011448),
            (4.7519794, -0.008730966, -0.011879999),
            (4.927977, -0.009402575, -0.012319999),
            (5.1071744, -0.0101122, -0.0127679985),
            (5.289572, -0.010861248, -0.013223998),
            (5.4751687, -0.011651152, -0.013687998),
            (5.663965, -0.012483371, -0.014159998),
            (5.8559613, -0.013359391, -0.014639998),
            (6.0511575, -0.014280722, -0.015127998),
            (6.249553, -0.015248898, -0.015623998),
            (6.4511485, -0.016265484, -0.016127998),
            (6.6559434, -0.017332062, -0.016639998),
            (6.863938, -0.018450249, -0.017159998),
            (7.075132, -0.019621681, -0.017687999),
            (7.2895255, -0.020848023, -0.018223999),
            (7.5071187, -0.022130962, -0.018768),
            (7.727911, -0.023472216, -0.01932),
            (7.951903, -0.024873523, -0.01988),
            (8.179094, -0.026336651, -0.020448),
            (8.409485, -0.02786339, -0.021023998),
            (8.643075, -0.02945556, -0.021607997),
            (8.879865, -0.031115, -0.022199996),
            (9.119853, -0.03284358, -0.022799995),
            (9.363041, -0.03464319, -0.023407994),
            (9.609427, -0.03651576, -0.024023993),
            (9.859014, -0.038463227, -0.024647992),
            (10.111798, -0.040487565, -0.025279991),
            (10.367783, -0.042590767, -0.02591999),
            (10.6269655, -0.044774856, -0.02656799),
            (10.889348, -0.04704188, -0.02722399),
            (11.154929, -0.049393915, -0.027887989),
            (11.423709, -0.051833052, -0.028559988),
            (11.695687, -0.05436142, -0.029239988),
            (11.970864, -0.05698117, -0.029927988),
            (12.24924, -0.05969447, -0.030623987),
            (12.530814, -0.062503524, -0.031327985),
            (12.815587, -0.06541056, -0.032039985),
            (13.103559, -0.06841783, -0.032759983),
            (13.394729, -0.071527615, -0.033487983),
            (13.689096, -0.07474221, -0.03422398),
            (13.986663, -0.07806395, -0.03496798),
            (14.287427, -0.08149518, -0.03571998),
            (14.59139, -0.08503829, -0.03647998),
            (14.89855, -0.088695675, -0.03724798),
            (15.208908, -0.092469774, -0.03802398),
            (15.522464, -0.09636304, -0.038807977),
            (15.839217, -0.10037795, -0.039599977),
        ];

        for _ in 0..100 {
            let expected = expected_poses[world.ticks as usize];
            update_world(&mut world);
            let pose = find::grid_pose(&world.grids, grid_id).unwrap().to_tuple();
            assert_eq!(pose, expected);
        }
    }

    #[test]
    fn snap_camera_to_local_planet() {
        let mut world = World::empty();

        world.target_camera.isometry.translation = Vec2::new(100.0, 300.0);
        world.snap_camera_to_local_planet = true;

        for _ in 0..100 {
            update_world(&mut world);
        }

        assert_eq!(
            world.camera.isometry.translation,
            Vec2::new(99.999985, 299.99994)
        );
        assert_eq!(world.camera.isometry.rotation, 2.8198416);
        assert_eq!(world.camera.zoom, 7.999999);
    }

    #[test]
    fn vehicle_arrives_at_its_destination() {
        // disclaimer: this is a very fragile test, and can be affected
        // by fuel requirements, changing ship design, etc.
        // I wouldn't be surprised if I have to get rid of it.
        // But it's good for now.

        let waypoint: Isometry2d = (600.0, 800.0, 0.5).into();

        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .spawn("pollux", Isometry2d::ZERO)
            .waypoint("pollux", waypoint)
            .build();

        let grid_id = find::grid_by_name(&world.grids, "pollux").unwrap();

        for _ in 0..20 {
            for _ in 0..1000 {
                update_world(&mut world);
            }

            let elapsed = apparent_elapsed_time(&world);
            let pose = find::grid_pose(&world.grids, grid_id).unwrap().to_tuple();
            println!(
                "{} ({:0.1}): {}, {}, {}",
                world.ticks,
                elapsed.as_secs_f64(),
                pose.0,
                pose.1,
                pose.2
            );
        }

        let pose = find::grid_pose(&world.grids, grid_id).unwrap();
        let error = pose.translation - waypoint.translation;

        assert!(error.x.abs() < 3.0);
        assert!(error.y.abs() < 3.0);
    }
}
