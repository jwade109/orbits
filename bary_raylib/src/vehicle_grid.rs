use crate::components::*;
use crate::computer::*;
use crate::light::*;
use crate::part::*;
use crate::thruster::*;
use crate::world::*;
use bary_core::prelude::*;

#[derive(Debug, Clone)]
pub struct VehicleGrid {
    pub name: String,
    pub parts_mass: Mass,
    pub isometry: Isometry2d,
    pub linear_velocity: Vec2,
    pub angular_velocity: f32,
    pub linear_acceleration: Vec2,
    pub angular_acceleration: f32,
    pub external_thrust: IVec2,
    pub parts: Vec<EntityId>,
    pub thrusters: Vec<EntityId>,
    pub computers: Vec<EntityId>,
    pub lights: Vec<EntityId>,
    pub requires_thruster_update: bool,
}

impl VehicleGrid {
    pub fn with_name(name: impl Into<String>) -> Self {
        VehicleGrid {
            name: name.into(),
            parts_mass: Mass::ZERO,
            linear_velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            linear_acceleration: Vec2::ZERO,
            angular_acceleration: 0.0,
            external_thrust: IVec2::ZERO,
            isometry: Isometry2d::default(),
            parts: Vec::new(),
            thrusters: Vec::new(),
            computers: Vec::new(),
            lights: Vec::new(),
            requires_thruster_update: false,
        }
    }
}

pub fn spawn_grid_from_blueprint(
    counter: &mut EntitySpawner,
    prototypes: &Components<(PartPrototype, MaybeTexture)>,
    grids: &mut Components<VehicleGrid>,
    parts: &mut Components<Part>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    lights: &mut Components<Light>,
    name: impl Into<String>,
    bp: &Blueprint,
) -> BaryResult<EntityId> {
    let grid = VehicleGrid::with_name(name);
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
    use super::*;

    /// Spawns a grid according to the given blueprint.
    /// Exclusive version of [`super::spawn_grid_from_blueprint`].
    pub fn spawn_grid_from_blueprint(
        world: &mut World,
        name: impl Into<String>,
        bp: &Blueprint,
    ) -> BaryResult<EntityId> {
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

    /// Spawns an empty grid with the given name.
    /// Exclusive version of [`super::spawn_empty_grid`].
    pub fn spawn_empty_grid(world: &mut World, name: &str) -> EntityId {
        super::spawn_empty_grid(&mut world.spawner, &mut world.grids, name)
    }

    /// Spawns a new grid according to a named blueprint.
    pub fn spawn_grid_by_name(world: &mut World, name: &str) -> BaryResult<EntityId> {
        let bp = find::blueprint_by_name(&world.blueprints, name)
            .ok_or(BaryError::BadBlueprint)?
            .clone();
        spawn_grid_from_blueprint(world, name, &bp)
    }

    /// Inserts a part into an existing grid.
    /// Exclusive version of [`super::insert_part`].
    pub fn insert_part(
        grid_id: EntityId,
        world: &mut World,
        instance: &PartInstance,
    ) -> BaryResult<EntityId> {
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
        thruster_id: EntityId,
        world: &mut World,
        new_state: bool,
    ) -> BaryResult<()> {
        super::set_thruster_state(
            thruster_id,
            &mut world.grids,
            &mut world.thrusters,
            &world.parts,
            new_state,
        )
    }
}

/// Spawns an empty vehicle grid.
pub fn spawn_empty_grid(
    spawner: &mut EntitySpawner,
    grids: &mut Components<VehicleGrid>,
    name: &str,
) -> EntityId {
    let grid = VehicleGrid::with_name(name);
    let id = spawner.spawn();
    grids.spawn(id, grid);
    id
}

pub fn insert_part(
    grid_id: EntityId,
    counter: &mut EntitySpawner,
    grids: &mut Components<VehicleGrid>,
    prototypes: &Components<(PartPrototype, MaybeTexture)>,
    parts: &mut Components<Part>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    lights: &mut Components<Light>,
    instance: &PartInstance,
) -> BaryResult<EntityId> {
    let grid = grids.try_get_mut(grid_id)?;
    let proto_id = find::part_by_name(prototypes, &instance.name).ok_or(BaryError::BadPartName)?;
    let (proto, _texture) = prototypes.try_get(proto_id)?;

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

    if let Some(data) = &proto.thruster_data {
        let thruster = Thruster {
            is_on: false,
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
            let light = Light::new(grid_id, proto_id, pos);
            lights.spawn(part_id, light);
            grid.lights.push(part_id);
        }
    }

    Ok(part_id)
}

pub fn despawn_grid(
    grid_id: EntityId,
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
    use super::*;

    pub fn blueprint_by_name<'a>(
        blueprints: &'a Components<NamedBlueprint>,
        name: &str,
    ) -> Option<&'a Blueprint> {
        blueprints
            .values()
            .find(|(n, _bp)| n == name)
            .map(|(_, bp)| bp)
    }

    pub fn part_by_name(
        prototypes: &Components<(PartPrototype, MaybeTexture)>,
        name: &str,
    ) -> Option<EntityId> {
        prototypes
            .iter()
            .find(|(_, (proto, _))| proto.part_name() == name)
            .map(|e| *e.0)
    }

    pub fn closest_grid(
        grids: &Components<VehicleGrid>,
        test_pos: Vec2,
    ) -> Option<(EntityId, Vec2)> {
        let mut best: Option<(EntityId, Vec2, f32)> = None;
        for (e, grid) in grids.iter() {
            let in_frame = express_in_frame(grid.isometry, test_pos);
            let dist = in_frame.length_squared();
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
    prototypes: &Components<(PartPrototype, MaybeTexture)>,
    grid_id: EntityId,
) -> BaryResult<Blueprint> {
    let grid = grids.try_get(grid_id)?;
    let mut bp = Blueprint::new();
    for part_id in &grid.parts {
        let part = parts.try_get(*part_id)?;
        let (proto, _texture) = prototypes.try_get(part.prototype)?;
        bp.add_part(proto.name.to_string(), part.placement, part.layer);
    }
    Ok(bp)
}

pub fn get_sum_linear_forces(
    grid_id: EntityId,
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

fn set_thruster_state(
    thruster_id: EntityId,
    grids: &mut Components<VehicleGrid>,
    thrusters: &mut Components<Thruster>,
    parts: &Components<Part>,
    new_state: bool,
) -> BaryResult<()> {
    let thruster = thrusters.try_get_mut(thruster_id)?;

    if new_state == thruster.is_on {
        return Ok(());
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

    Ok(())
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

        let (id, (proto, _texture)) = iter.next().unwrap();
        assert_eq!(*id, EntityId(0));
        assert_eq!(proto.part_name(), "angled-frame");

        let (id, (proto, _texture)) = iter.next().unwrap();
        assert_eq!(*id, EntityId(1));
        assert_eq!(proto.part_name(), "antenna");

        let (id, (proto, _texture)) = iter.next().unwrap();
        assert_eq!(*id, EntityId(2));
        assert_eq!(proto.part_name(), "battery");

        let (id, (proto, _texture)) = iter.next().unwrap();
        assert_eq!(*id, EntityId(3));
        assert_eq!(proto.part_name(), "cargo");

        let (id, (proto, _texture)) = iter.next().unwrap();
        assert_eq!(*id, EntityId(4));
        assert_eq!(proto.part_name(), "chemical-plant");

        let (id, (proto, _texture)) = iter.next().unwrap();
        assert_eq!(*id, EntityId(5));
        assert_eq!(proto.part_name(), "container");

        let (id, (proto, _texture)) = iter.next().unwrap();
        assert_eq!(*id, EntityId(6));
        assert_eq!(proto.part_name(), "cpu");

        let (id, (proto, _texture)) = iter.next().unwrap();
        assert_eq!(*id, EntityId(7));
        assert_eq!(proto.part_name(), "debug-item-source");

        let (id, (proto, _texture)) = iter.next().unwrap();
        assert_eq!(*id, EntityId(8));
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

        let expected_grid_id = EntityId(34);

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
        assert_eq!(*id, EntityId(58));
        assert_eq!(cpu.grid_id, expected_grid_id);
        assert_eq!(cpu.prototype, EntityId(6));

        // get the prototype definition for the computer
        let (proto, _texture) = world.prototypes.get(cpu.prototype).unwrap();

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

        assert!(find::closest_grid(&world.grids, Vec2::new(100.0, 200.0)).is_none());

        let id = world::spawn_grid_by_name(&mut world, "remora").unwrap();
        assert_eq!(id, EntityId(34));

        let grid = world.grids.try_get_mut(id).unwrap();
        grid.isometry.translation = (40.0, 156.0).into();

        for _ in 0..100 {
            update_world(&mut world, (1080.0, 720.0).into(), None);
            let e = find::closest_grid(&world.grids, Vec2::new(100.0, 200.0));
            assert_eq!(e, Some((EntityId(34), Vec2::new(60.0, 44.0))));
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

        let (proto, _texture) = world.prototypes.try_get(proto_id).unwrap();
        let dims = proto.dims;

        assert_eq!(proto_id, EntityId(16));

        let grid_id = world::spawn_grid_by_name(&mut world, "pollux").unwrap();

        assert_eq!(world.parts.len(), 98);
        assert_eq!(world.thrusters.len(), 18);

        assert_eq!(grid_id, EntityId(31));

        let instance = PartInstance::new(
            part_name,
            PartLayer::Internal,
            GridPlacement::new((2, 3), Rotation::East, dims),
        );

        let id = world::insert_part(grid_id, &mut world, &instance).unwrap();

        assert_eq!(id, EntityId(130));

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

        let result = world::insert_part(EntityId(103), &mut world, &instance);

        assert_eq!(result, Err(BaryError::EntityNotFound));
    }

    #[test]
    fn set_thruster_state() {
        let mut world = WorldBuilder::new().assets("../assets/").build();

        let grid_id = world::spawn_empty_grid(&mut world, "whatever");

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts.len(), 0);
        assert_eq!(grid.parts_mass, Mass::ZERO);

        assert_eq!(grid_id, EntityId(30));

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

        assert_eq!(a_id, EntityId(31));
        assert_eq!(b_id, EntityId(32));

        let r1 = world::set_thruster_state(a_id, &mut world, true);
        let r2 = world::set_thruster_state(b_id, &mut world, true);

        assert_eq!(r1, Ok(()));
        assert_eq!(r2, Ok(()));

        let sum =
            get_sum_linear_forces(grid_id, &world.grids, &world.parts, &world.thrusters).unwrap();

        assert_eq!(sum.x, 400000.0);
        assert_eq!(sum.y, 320000.0);

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.external_thrust, IVec2::new(400000000, 320000000));

        let r1 = world::set_thruster_state(a_id, &mut world, false);
        let r2 = world::set_thruster_state(b_id, &mut world, true);

        assert_eq!(r1, Ok(()));
        assert_eq!(r2, Ok(()));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.external_thrust, IVec2::new(0, 320000000));

        let r1 = world::set_thruster_state(a_id, &mut world, false);
        let r2 = world::set_thruster_state(b_id, &mut world, false);

        assert_eq!(r1, Ok(()));
        assert_eq!(r2, Ok(()));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.external_thrust, IVec2::new(0, 0));
    }
}
