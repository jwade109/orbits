use bevy::prelude::*;
use bevy_ecs::{query::QueryEntityError, system::SystemParam};
use game::starling::{parts::PartLayer, vehicle::Blueprint};

use crate::game_version_two::target_docking_transform;

use super::*;

#[derive(SystemParam)]
pub struct Spacecraft<'w, 's> {
    grids: Query<
        'w,
        's,
        (
            &'static mut SpacecraftGrid,
            &'static Children,
            &'static mut Transform,
        ),
    >,
    parts: Query<
        'w,
        's,
        (
            &'static PartInstance,
            &'static ChildOf,
            &'static mut Transform,
        ),
        Without<SpacecraftGrid>,
    >,
}

impl<'w, 's> Spacecraft<'w, 's> {
    pub fn grid_exists(&self, e: Entity) -> bool {
        self.grids.contains(e)
    }

    pub fn get_vehicle_from_part_id(&self, part: Entity) -> Result<Entity, QueryEntityError> {
        let (_, child_of, _) = self.parts.get(part)?;
        Ok(child_of.0)
    }

    pub fn grid_transform(&self, grid: Entity) -> Result<Transform, QueryEntityError> {
        let (_, _, transform) = self.grids.get(grid)?;
        Ok(*transform)
    }

    pub fn blueprint(&self, e: Entity) -> Result<Blueprint, QueryEntityError> {
        let mut blueprint = Blueprint::new();
        let (_, children, _) = self.grids.get(e)?;
        for c in children {
            let (part, _, _) = self.parts.get(*c)?;
            blueprint.add_part(part.proto.clone(), part.pos, part.rot);
        }
        Ok(blueprint)
    }

    pub fn get_part_at(&self, coords: GridCoord) -> Option<Entity> {
        let (_, children, _) = self.grids.get(coords.entity).ok()?;

        if children.is_empty() {
            warn!("Empty grid!");
            return None;
        }

        // TODO this is very bugged.
        for id in children {
            let (part, _, _) = self.parts.get(*id).ok()?;
            if part.prototype().layer() != PartLayer::Internal {
                continue;
            }
            let offset = (coords.coord - part.origin()).inner();
            let dims = part.dims.as_ivec2();
            if offset.x >= 0 && offset.y >= 0 && offset.x <= dims.x && offset.y <= dims.y {
                return Some(*id);
            }
        }

        None
    }

    pub fn get_merge_transform(
        &self,
        host_part: Entity,
        target_part: Entity,
    ) -> Result<Transform, QueryEntityError> {
        let host_vehicle = self.get_vehicle_from_part_id(host_part)?;
        let target_vehicle = self.get_vehicle_from_part_id(target_part)?;

        let (port, parent, port_a) = self.parts.get(host_part)?;
        let (other_port, other_parent, port_b) = self.parts.get(target_part)?;

        let (grid, children, ownship_root) = self.grids.get(parent.0)?;

        let ownship_root = ownship_root.clone();
        let velocity = grid.velocity;
        let angular_velocity = grid.angular_velocity;

        let additional_velocity = port_a
            .translation
            .cross((Vec3::Z * angular_velocity as f32))
            .xy()
            .as_dvec2();

        let mut port_a = *port_a;
        let mut port_b = *port_b;

        let off_a = port.prototype().dims_meters().x;
        let off_b = other_port.prototype().dims_meters().x;

        port_a.translation += port_a.right() * off_a / 2.0;
        port_b.translation += port_b.right() * off_b / 2.0;

        let (grid, _, target_root) = self.grids.get(other_parent.0)?;

        Ok(target_docking_transform(ownship_root, port_a, port_b))
    }
}
