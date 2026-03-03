use crate::components::*;
use crate::computer::*;
use crate::light::*;
use crate::part::*;
use crate::result::*;
use crate::thruster::*;
use crate::vehicle_grid::*;
use crate::world::*;
use bary_core::prelude::*;
use log::{debug, info};

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
    grids.get_mut(grid_id).ok_or(BaryError::EntityNotFound)?;
    for (_id, proto) in bp.parts() {
        insert_part(
            grid_id, counter, grids, prototypes, parts, thrusters, computers, lights, proto,
        )?;
    }
    Ok(grid_id)
}

pub mod world {
    use crate::ring_particle::PingParticle;

    use super::*;

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

    pub fn set_grid_isometry(world: &mut World, grid_id: Ent, iso: Isometry2d) -> BaryResult<()> {
        info!("Setting isometry of grid {} to {:?}", grid_id, iso);
        let grid = world.grids.try_get_mut(grid_id)?;
        grid.isometry = iso;
        Ok(())
    }

    /// Spawns an empty grid with the given name.
    /// Exclusive version of [`super::spawn_empty_grid`].
    pub fn spawn_empty_grid(world: &mut World, name: &str) -> Ent {
        super::spawn_empty_grid(&mut world.spawner, &mut world.grids, name)
    }

    /// Spawns a new grid according to a named blueprint.
    pub fn spawn_grid_by_name(world: &mut World, name: &str) -> BaryResult<Ent> {
        let bp = find::blueprint_by_name(&world.blueprints, name)
            .ok_or(BaryError::BadBlueprint)?
            .clone();
        spawn_grid_from_blueprint(world, name, &bp)
    }

    /// Inserts a part into an existing grid.
    /// Exclusive version of [`super::insert_part`].
    pub fn insert_part(
        grid_id: Ent,
        world: &mut World,
        instance: &PartInstance,
    ) -> BaryResult<Ent> {
        super::insert_part(
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

    /// Sets the state of a given thruster, modifying related quantities
    /// in the root grid if necessary.
    /// Exclusive version of [`super::set_thruster_state`].
    pub fn set_thruster_state(
        thruster_id: Ent,
        world: &mut World,
        new_state: bool,
    ) -> BaryResult<bool> {
        super::set_thruster_state(
            thruster_id,
            &mut world.grids,
            &mut world.thrusters,
            &world.parts,
            new_state,
        )
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
pub fn spawn_empty_grid(
    spawner: &mut EntitySpawner,
    grids: &mut Components<VehicleGrid>,
    name: &str,
) -> Ent {
    debug!("Spawning empty grid with name {}", name);
    let grid = VehicleGrid::with_name(name);
    let id = spawner.spawn();
    grids.spawn(id, grid);
    id
}

pub fn insert_part(
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
    debug!("Inserting part {} into grid {}", instance.name, grid_id);
    let grid = grids.try_get_mut(grid_id)?;
    let proto_id = find::part_by_name(prototypes, &instance.name).ok_or(BaryError::BadPartName)?;
    let proto = prototypes.try_get(proto_id)?;

    grid.parts_mass += proto.mass;

    let part = Part {
        placement: instance.placement,
        layer: instance.layer(),
        prototype: proto_id,
        grid_id,
    };

    let part_id = counter.spawn();

    grid.parts.push(part_id);
    parts.spawn(part_id, part);

    grid.mark_occupied(instance.placement, instance.layer(), part_id);

    if let Some(data) = &proto.thruster_data {
        let thruster = Thruster {
            is_on: false,
            is_rcs: data.is_rcs,
            thrust_millinewtons: (data.thrust * 1000.0).round() as i32,
            prototype: proto_id,
            grid_id,
        };
        thrusters.spawn(part_id, thruster);
        grid.thrusters.push(part_id);
    }
    if let Some(_data) = &proto.computer_data {
        let cpu = Computer::new(grid_id, proto_id);
        computers.spawn(part_id, cpu);
        grid.computers.push(part_id);
    }
    if let Some(data) = &proto.thruster_data {
        if data.is_rcs {
            let pos = instance.placement.center_isometry().translation;
            let light_idx = lights.len();
            let light = Light::new(grid_id, proto_id, pos, light_idx as u32);
            lights.spawn(part_id, light);
            grid.lights.push(part_id);
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

    pub fn part_by_name(prototypes: &Components<PartPrototype>, name: &str) -> Option<Ent> {
        let result = prototypes
            .iter()
            .find(|(_, proto)| proto.part_name() == name)
            .map(|e| *e.0);

        if result.is_none() {
            error!("Failed to get part with name {}", name);
        }

        result
    }

    pub fn closest_grid(
        grids: &Components<VehicleGrid>,
        test_pos: Vec2,
        dist_limit: impl Into<Option<f32>>,
    ) -> Option<(Ent, Vec2)> {
        let mut best: Option<(Ent, Vec2, f32)> = None;
        let dist_limit = dist_limit.into().unwrap_or(std::f32::INFINITY);
        for (e, grid) in grids.iter() {
            let in_frame = express_in_frame(grid.isometry, test_pos);
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
        let thrust = thruster.thrust_millinewtons as f32 / 1000.0;
        let thrust = rotate(Vec2::X, part.placement.rot().to_angle() as f32) * thrust;
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
    grids: &mut Components<VehicleGrid>,
    thrusters: &mut Components<Thruster>,
    parts: &Components<Part>,
    new_state: bool,
) -> BaryResult<bool> {
    let thruster = thrusters.try_get_mut(thruster_id)?;

    if new_state == thruster.is_on {
        return Ok(false);
    }

    let part = parts.try_get(thruster_id)?;
    let grid = grids.try_get_mut(thruster.grid_id)?;

    thruster.is_on = new_state;

    let dir = match part.placement.rot() {
        Rotation::East => IVec2::X,
        Rotation::North => IVec2::Y,
        Rotation::West => -IVec2::X,
        Rotation::South => -IVec2::Y,
    };

    let mul = if new_state { 1 } else { -1 };
    let thrust_vec = mul * dir * thruster.thrust_millinewtons;
    grid.external_thrust += thrust_vec;

    Ok(true)
}

// TODO(testing)
pub fn get_parts_at<'a>(
    grid: &VehicleGrid,
    parts: &'a Components<Part>,
    coord: PartCoord,
) -> Vec<(Ent, &'a Part)> {
    // TODO(optimization) can make VehicleGrid keep a spatial LUT for this
    let mut ret = Vec::new();
    for part_id in &grid.parts {
        let Ok(part) = parts.try_get(*part_id) else {
            continue;
        };

        if !part.placement.contains(coord) {
            continue;
        }

        ret.push((*part_id, part));
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_builder::WorldBuilder;

    #[test]
    fn part_prototypes() {
        let world = WorldBuilder::new()
            .assets("../assets")
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
    }

    #[test]
    fn vehicle_spawning_and_despawning() {
        let mut world = WorldBuilder::new()
            .assets("../assets")
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

        assert_eq!(result, Err(BaryError::EntityNotFound));
    }

    #[test]
    fn nearest_grid() {
        let mut world = WorldBuilder::new()
            .assets("../assets")
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        assert!(find::closest_grid(&world.grids, Vec2::new(100.0, 200.0), None).is_none());

        let id = world::spawn_grid_by_name(&mut world, "remora").unwrap();
        assert_eq!(id, Ent(34));

        let grid = world.grids.try_get_mut(id).unwrap();
        grid.isometry.translation = (40.0, 156.0).into();

        for _ in 0..100 {
            update_world(&mut world);
            let test_pos = Vec2::new(100.0, 200.0);
            let e = find::closest_grid(&world.grids, test_pos, None);
            assert_eq!(e, Some((Ent(34), Vec2::new(60.0, 44.0))));
        }
    }

    #[test]
    fn insert_parts() {
        let mut world = WorldBuilder::new()
            .assets("../assets")
            .blueprint("pollux")
            .build();

        let part_name = "motor";

        let proto_id = find::part_by_name(&world.prototypes, part_name).unwrap();

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
    }

    #[test]
    fn parts_mass() {
        let mut world = WorldBuilder::new()
            .assets("../assets")
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
    }

    #[test]
    fn calculate_blueprints() {
        let mut world = WorldBuilder::new()
            .assets("../assets")
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
    }

    #[test]
    fn bad_part_insertion() {
        let mut world = WorldBuilder::new().assets("../assets/").build();
        let id = world::spawn_empty_grid(&mut world, "whatever");

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

        assert_eq!(result, Err(BaryError::EntityNotFound));
    }

    #[test]
    fn set_thruster_state() {
        let mut world = WorldBuilder::new().assets("../assets/").build();

        let grid_id = world::spawn_empty_grid(&mut world, "whatever");

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
        assert_eq!(grid.parts, vec![a_id, b_id]);
        assert_eq!(grid.thrusters, vec![a_id, b_id]);
        assert_eq!(grid.parts_mass, Mass::grams(870000));

        assert_eq!(a_id, Ent(31));
        assert_eq!(b_id, Ent(32));

        let r1 = world::set_thruster_state(a_id, &mut world, true);
        let r2 = world::set_thruster_state(b_id, &mut world, true);

        assert_eq!(r1, Ok(true));
        assert_eq!(r2, Ok(true));

        let sum =
            get_sum_linear_forces(grid_id, &world.grids, &world.parts, &world.thrusters).unwrap();

        assert_eq!(sum.x, 400000.0);
        assert_eq!(sum.y, 320000.0);

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.external_thrust, IVec2::new(400000000, 320000000));

        let r1 = world::set_thruster_state(a_id, &mut world, false);
        let r2 = world::set_thruster_state(b_id, &mut world, true);

        assert_eq!(r1, Ok(true));
        assert_eq!(r2, Ok(false));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.external_thrust, IVec2::new(0, 320000000));

        let r1 = world::set_thruster_state(a_id, &mut world, false);
        let r2 = world::set_thruster_state(b_id, &mut world, false);

        assert_eq!(r1, Ok(false));
        assert_eq!(r2, Ok(true));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.external_thrust, IVec2::new(0, 0));
    }

    #[test]
    fn parts_center_of_mass() {
        let mut world = WorldBuilder::new()
            .assets("../assets/")
            .blueprint("pollux")
            .build();

        let id = world::spawn_grid_by_name(&mut world, "pollux").unwrap();
        let com = get_parts_center_of_mass(id, &mut world).unwrap();

        // TODO is this right? possibly. seems close enough
        assert_eq!(com, Vec2::new(0.001067417, 0.022271877));

        let cargo_id = find::part_by_name(&world.prototypes, "cargo").unwrap();
        let cargo_proto = world.prototypes.try_get(cargo_id).unwrap();

        assert_eq!(cargo_proto.dims, (6, 6).into());
        assert_eq!(cargo_proto.dims_meters(), (1.5, 1.5).into());

        let instance = PartInstance::from_prototype(cargo_proto, (0, 0).into(), Rotation::East);

        let grid_id = world::spawn_empty_grid(&mut world, "whatever");
        _ = world::insert_part(grid_id, &mut world, &instance);

        let com = get_parts_center_of_mass(grid_id, &mut world).unwrap();

        assert_eq!(com, Vec2::splat(0.75));
    }

    #[test]
    fn pure_linear_acceleration() {
        let mut world = WorldBuilder::new().assets("../assets/").build();

        // modifying the prototype for motor so it has easy quantities
        let proto_id = find::part_by_name(&world.prototypes, "motor").unwrap();
        let proto = world.prototypes.try_get_mut(proto_id).unwrap();

        proto.mass = Mass::kilograms(1000);
        if let Some(t) = &mut proto.thruster_data {
            // 3500 newtons
            t.thrust = 3500.0;
        }

        let grid_id = world::spawn_empty_grid(&mut world, "testbed");

        let instance = PartInstance {
            name: "motor".to_string(),
            layer: PartLayer::Internal,
            placement: GridPlacement::new((0, 0), Rotation::East, (6, 3)),
        };

        let thruster_id = world::insert_part(grid_id, &mut world, &instance).unwrap();

        // obviously, turn the main thruster on
        let r = world::set_thruster_state(thruster_id, &mut world, true);
        assert_eq!(r, Ok(true));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.external_thrust, IVec2::new(3500000, 0));
        assert_eq!(grid.parts_mass, Mass::kilograms(1000));

        // body frame acceleration should be 3.5 m/s^2
        assert_eq!(grid.linear_acceleration(), Vec2::new(3.5, 0.0));

        // run the simulation for 2 seconds at 50 Hz
        for _ in 0..100 {
            update_world(&mut world);
        }

        let iso = world.grids.try_get(grid_id).unwrap().isometry;

        // this is an approximation of the following
        // continuous time kinematic equation:
        // d = 1/2 at^2  --> 0.5 * 3.5 * 2^2 = 7
        assert_eq!(iso.translation, Vec2::new(6.9299994, 0.0));
    }

    #[cfg(test)]
    mod tests {
        use super::*;

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
    }
}
