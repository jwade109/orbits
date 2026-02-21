use crate::components::*;
use crate::computer::*;
use crate::light::*;
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
    pub parts: Vec<(GridPlacement, EntityId)>,
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
    name: String,
    bp: &Blueprint,
    prototypes: &Components<(PartPrototype, MaybeTexture)>,
) -> Option<VehicleGrid> {
    let mut initial_mass = Mass::ZERO;
    let mut parts = Vec::new();
    for part in bp.parts() {
        let e = find_part_by_name(prototypes, &part.1.name)?;
        let proto = prototypes.get_with_log(e)?;
        initial_mass += proto.0.mass;
        let placement = part.1.placement;
        parts.push((placement, e));
    }
    Some(VehicleGrid {
        name,
        mass: initial_mass,
        linear_velocity: Vec2::ZERO,
        angular_velocity: 0.1,
        blueprint: bp.clone(),
        isometry: Isometry2d::default(),
        parts,
        thrusters: Vec::new(),
        computers: Vec::new(),
        lights: Vec::new(),
    })
}

pub fn spawn_grid_from_blueprint(
    counter: &mut EntityCounter,
    prototypes: &Components<(PartPrototype, MaybeTexture)>,
    grids: &mut Components<VehicleGrid>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    lights: &mut Components<Light>,
    pos: Vec2,
    name: String,
    bp: &Blueprint,
) -> BaryResult<EntityId> {
    let mut grid = grid_from_blueprint(name, bp, &prototypes).ok_or(BaryError::BadBlueprint)?;
    grid.isometry.translation = pos;
    grid.linear_velocity = randvec(0.1, 3.0);

    let grid_id = counter.get_id();
    grids.spawn(grid_id, grid.clone());
    let spawned_grid = grids.get_mut(grid_id).ok_or(BaryError::EntityNotFound)?;

    for (placement, prototype_id) in &grid.parts {
        let part_id = counter.get_id();
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
        if chance(0.1) {
            let pos = placement.center_isometry().translation;
            let light = Light::new(grid_id, *prototype_id, pos);
            println!("Light: {} {} {}", grid_id, *prototype_id, part_id);
            lights.spawn(part_id, light);
            spawned_grid.lights.push(part_id);
        }
    }
    Ok(grid_id)
}

pub fn despawn_grid(
    grid_id: EntityId,
    grids: &mut Components<VehicleGrid>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
) -> BaryResult<()> {
    let grid = grids.despawn(grid_id)?;
    for id in grid.thrusters {
        thrusters.despawn(id)?;
    }
    for id in grid.computers {
        computers.despawn(id)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios::with_vehicle_data_loaded;

    #[test]
    fn test_part_prototypes() {
        let world = with_vehicle_data_loaded("../assets/");

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
    fn test_vehicle_spawning_and_despawning() {
        let mut world = with_vehicle_data_loaded("../assets/");

        let name = "pollux";

        // get the blueprint for the pollux
        let bp = find_blueprint_by_name(&world.blueprints, name).expect("Expected a blueprint");

        // spawn that vehicle using its blueprint
        let grid_id = spawn_grid_from_blueprint(
            &mut world.counter,
            &world.prototypes,
            &mut world.grids,
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
        assert_eq!(world.thrusters.len(), 18);
        assert_eq!(world.computers.len(), 1);

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
            &mut world.thrusters,
            &mut world.computers,
        );
        assert_eq!(result, Ok(()));

        // now the world should be empty
        assert_eq!(world.grids.len(), 0);
        assert_eq!(world.thrusters.len(), 0);
        assert_eq!(world.computers.len(), 0);
    }
}
