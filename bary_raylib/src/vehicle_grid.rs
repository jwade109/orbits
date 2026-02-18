use crate::components::*;
use crate::computer::*;
use crate::thruster::*;
use crate::world::*;
use bary_core::prelude::*;

#[derive(Debug, Clone)]
pub struct VehicleGrid {
    pub mass: Mass,
    pub isometry: Isometry2d,
    pub linear_velocity: Vec2,
    pub angular_velocity: f32,
    pub blueprint: Blueprint,
    pub parts: Vec<(GridPlacement, EntityId)>,
    pub thrusters: Vec<EntityId>,
    pub computers: Vec<EntityId>,
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
        mass: initial_mass,
        linear_velocity: Vec2::ZERO,
        angular_velocity: 0.0,
        blueprint: bp.clone(),
        isometry: Isometry2d::default(),
        parts,
        thrusters: Vec::new(),
        computers: Vec::new(),
    })
}

pub fn spawn_grid_from_blueprint(
    prototypes: &Components<(PartPrototype, MaybeTexture)>,
    grids: &mut Components<VehicleGrid>,
    thrusters: &mut Components<Thruster>,
    computers: &mut Components<Computer>,
    pos: Vec2,
    bp: &Blueprint,
) -> Option<EntityId> {
    let mut grid = grid_from_blueprint(bp, &prototypes)?;
    grid.isometry.translation = pos;
    grid.linear_velocity = randvec(3.0, 12.0);
    let grid_id = grids.spawn(grid.clone());
    let spawned_grid = grids.get_mut(grid_id)?;
    for (_placement, part_id) in &grid.parts {
        let (part, _texture) = prototypes.get_with_log(*part_id)?;
        if let Some(_data) = &part.thruster_data {
            let thruster_id = thrusters.spawn(Thruster {
                prototype: *part_id,
                grid_id,
            });
            spawned_grid.thrusters.push(thruster_id);
        }
        if let Some(_data) = &part.computer_data {
            let computer_id = computers.spawn(Computer {
                prototype: *part_id,
                grid_id,
            });
            spawned_grid.computers.push(computer_id);
        }
    }
    Some(grid_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios::with_vehicle_data_loaded;

    #[test]
    fn test_vehicle_spawning_and_despawning() {
        let mut world = with_vehicle_data_loaded("../assets/");

        let (_name, bp) = world.blueprints.get(EntityId(0)).unwrap();

        let grid_id = spawn_grid_from_blueprint(
            &world.prototypes,
            &mut world.grids,
            &mut world.thrusters,
            &mut world.computers,
            Vec2::ZERO,
            &bp,
        )
        .unwrap();

        // this is the first grid in the world
        assert_eq!(grid_id, EntityId(0));

        assert_eq!(world.grids.len(), 1);
        assert_eq!(world.thrusters.len(), 18);
        assert_eq!(world.computers.len(), 1);

        let result = despawn_grid(
            grid_id,
            &mut world.grids,
            &mut world.thrusters,
            &mut world.computers,
        );

        assert_eq!(result, Ok(()));

        assert_eq!(world.grids.len(), 0);
        assert_eq!(world.thrusters.len(), 0);
        assert_eq!(world.computers.len(), 0);
    }
}
