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
    pub mass: Mass,
    pub isometry: Isometry2d,
    pub linear_velocity: Vec2,
    pub angular_velocity: f32,
    pub blueprint: Blueprint,
    #[deprecated]
    pub layout: Vec<(GridPlacement, EntityId)>,
    pub parts: Vec<EntityId>,
    pub thrusters: Vec<EntityId>,
    pub computers: Vec<EntityId>,
    pub lights: Vec<EntityId>,
}

fn find_part_by_name(
    prototypes: &Components<(PartPrototype, MaybeTexture)>,
    name: &str,
) -> Option<EntityId> {
    prototypes
        .iter()
        .find(|(_, (proto, _))| proto.part_name() == name)
        .map(|e| *e.0)
}

pub fn grid_from_blueprint(
    name: impl Into<String>,
    bp: &Blueprint,
    prototypes: &Components<(PartPrototype, MaybeTexture)>,
) -> Option<VehicleGrid> {
    let mut initial_mass = Mass::ZERO;
    let mut layout = Vec::new();
    for part in bp.parts() {
        let e = find_part_by_name(prototypes, &part.1.name)?;
        let proto = prototypes.get_with_log(e)?;
        initial_mass += proto.0.mass;
        let placement = part.1.placement;
        layout.push((placement, e));
    }
    Some(VehicleGrid {
        name: name.into(),
        mass: initial_mass,
        linear_velocity: Vec2::ZERO,
        angular_velocity: 0.0,
        blueprint: bp.clone(),
        isometry: Isometry2d::default(),
        layout,
        parts: Vec::new(),
        thrusters: Vec::new(),
        computers: Vec::new(),
        lights: Vec::new(),
    })
}

/// Spawns a grid which matches the given blueprint.
/// This function requires exclusive world access.
/// Use [`spawn_grid_from_blueprint`] if you need
/// a less demanding borrow.
pub fn spawn_grid_from_blueprint_world(
    world: &mut World,
    pos: Vec2,
    name: impl Into<String>,
    bp: &Blueprint,
) -> BaryResult<EntityId> {
    spawn_grid_from_blueprint(
        &mut world.counter,
        &mut world.prototypes,
        &mut world.grids,
        &mut world.parts,
        &mut world.thrusters,
        &mut world.computers,
        &mut world.lights,
        pos,
        name,
        bp,
    )
}

pub fn spawn_grid_from_blueprint(
    counter: &mut EntityCounter,
    prototypes: &Components<(PartPrototype, MaybeTexture)>,
    grids: &mut Components<VehicleGrid>,
    parts: &mut Components<Part>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    lights: &mut Components<Light>,
    pos: Vec2,
    name: impl Into<String>,
    bp: &Blueprint,
) -> BaryResult<EntityId> {
    let mut grid = grid_from_blueprint(name, bp, &prototypes).ok_or(BaryError::BadBlueprint)?;
    grid.isometry.translation = pos;

    grid.linear_velocity = randvec(0.1, 3.0);
    grid.angular_velocity = rand(-0.1, 0.1);

    let grid_id = counter.get_id();
    grids.spawn(grid_id, grid.clone());
    let spawned_grid = grids.get_mut(grid_id).ok_or(BaryError::EntityNotFound)?;

    for (placement, prototype_id) in &grid.layout {
        let part_id = counter.get_id();

        let part_entity = Part {
            placement: *placement,
            prototype: *prototype_id,
            grid_id,
        };

        parts.spawn(part_id, part_entity);
        spawned_grid.parts.push(part_id);

        let (part, _texture) = prototypes
            .get_with_log(*prototype_id)
            .ok_or(BaryError::EntityNotFound)?;
        if let Some(_data) = &part.thruster_data {
            let thruster = Thruster {
                prototype: *prototype_id,
                grid_id,
            };
            thrusters.spawn(part_id, thruster);
            spawned_grid.thrusters.push(part_id);
        }
        if let Some(_data) = &part.computer_data {
            let cpu = Computer::new(grid_id, *prototype_id);
            computers.spawn(part_id, cpu);
            spawned_grid.computers.push(part_id);
        }
        if let Some(data) = &part.thruster_data {
            if !data.is_rcs {
                continue;
            }
            let pos = placement.center_isometry().translation;
            let light = Light::new(grid_id, *prototype_id, pos);
            lights.spawn(part_id, light);
            spawned_grid.lights.push(part_id);
        }
    }
    Ok(grid_id)
}

pub fn spawn_grid_by_name_world(world: &mut World, name: &str) -> BaryResult<EntityId> {
    let bp = find_blueprint_by_name(&world.blueprints, name)
        .ok_or(BaryError::BadBlueprint)?
        .clone();
    spawn_grid_from_blueprint_world(world, Vec2::ZERO, name, &bp)
}

pub fn insert_part_world(grid_id: EntityId, world: &mut World, name: &str) -> BaryResult<EntityId> {
    insert_part(
        grid_id,
        &mut world.counter,
        &mut world.grids,
        &mut world.prototypes,
        &mut world.parts,
        &mut world.thrusters,
        &mut world.computers,
        &mut world.lights,
        name,
    )
}

pub fn insert_part(
    grid_id: EntityId,
    counter: &mut EntityCounter,
    grids: &mut Components<VehicleGrid>,
    prototypes: &Components<(PartPrototype, MaybeTexture)>,
    parts: &mut Components<Part>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    lights: &mut Components<Light>,
    name: &str,
) -> BaryResult<EntityId> {
    let grid = grids.try_get_mut(grid_id)?;
    let proto_id = find_part_by_name(prototypes, name).ok_or(BaryError::BadPartName)?;
    let (proto, _texture) = prototypes.try_get(proto_id)?;
    let placement = GridPlacement::new((0, 0), Rotation::East, proto.dims);

    let part = Part {
        placement,
        prototype: proto_id,
        grid_id,
    };

    let part_id = counter.get_id();

    grid.parts.push(part_id);
    parts.spawn(part_id, part);

    if let Some(_data) = &proto.thruster_data {
        let thruster = Thruster {
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
            let pos = placement.center_isometry().translation;
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

pub fn find_blueprint_by_name<'a>(
    blueprints: &'a Components<NamedBlueprint>,
    name: &str,
) -> Option<&'a Blueprint> {
    blueprints
        .values()
        .find(|(n, _bp)| n == name)
        .map(|(_, bp)| bp)
}

pub fn express_in_frame(frame: Isometry2d, point: Vec2) -> Vec2 {
    let delta = point - frame.translation;
    let x = frame.local_x().dot(delta);
    let y = frame.local_y().dot(delta);
    (x, y).into()
}

pub fn find_closest_grid(
    grids: &Components<VehicleGrid>,
    test_pos: Vec2,
) -> Option<(EntityId, Vec2)> {
    let mut best: Option<(EntityId, Vec2, f32)> = None;
    for (e, grid) in grids.iter() {
        let in_frame = express_in_frame(grid.isometry, test_pos);
        let dist = in_frame.distance_squared(grid.isometry.translation);
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
        let bp = find_blueprint_by_name(&world.blueprints, name).expect("Expected a blueprint");

        // spawn that vehicle using its blueprint
        let grid_id = spawn_grid_from_blueprint(
            &mut world.counter,
            &world.prototypes,
            &mut world.grids,
            &mut world.parts,
            &mut world.thrusters,
            &mut world.computers,
            &mut world.lights,
            Vec2::ZERO,
            name.to_string(),
            &bp,
        )
        .expect("Expected the grid ID");

        let expected_grid_id = EntityId(34);

        // this entity should be the same every time
        assert_eq!(grid_id, expected_grid_id);

        // the mass should already be computed
        let grid = world.grids.get(expected_grid_id).unwrap();
        assert_eq!(grid.mass, Mass::grams(35134000));

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

        let e = find_closest_grid(&world.grids, Vec2::new(100.0, 200.0));

        assert!(e.is_none());

        let bp = find_blueprint_by_name(&world.blueprints, "remora")
            .unwrap()
            .clone();

        let id = spawn_grid_from_blueprint_world(&mut world, Vec2::new(40.0, 156.0), "remora", &bp);

        assert_eq!(id, Ok(EntityId(34)));

        let e = find_closest_grid(&world.grids, Vec2::new(100.0, 200.0));

        assert_eq!(e, Some((EntityId(34), Vec2::new(60.0, 44.0))));
    }

    #[test]
    fn insert_parts() {
        let mut world = WorldBuilder::new()
            .assets("../assets")
            .blueprint("pollux")
            .build();

        let part_name = "motor";

        let proto_id = find_part_by_name(&world.prototypes, part_name).unwrap();

        assert_eq!(proto_id, EntityId(16));

        let grid_id = spawn_grid_by_name_world(&mut world, "pollux").unwrap();

        assert_eq!(world.parts.len(), 98);
        assert_eq!(world.thrusters.len(), 18);

        assert_eq!(grid_id, EntityId(31));

        let id = insert_part_world(grid_id, &mut world, part_name).unwrap();

        assert_eq!(id, EntityId(130));

        let part = world.parts.get(id).unwrap();

        assert_eq!(part.grid_id, grid_id);
        assert_eq!(part.prototype, proto_id);
        assert_eq!(
            part.placement,
            // TODO allow insertion at a given placement
            GridPlacement::new((0, 0), Rotation::East, (6, 3))
        );

        assert_eq!(world.parts.len(), 99);
        assert_eq!(world.thrusters.len(), 19);
    }
}
