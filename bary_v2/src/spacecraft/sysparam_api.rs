use bary_core::prelude::*;
use bevy::prelude::*;
use bevy_ecs::{query::QueryEntityError, system::SystemParam};

use crate::target_docking_transform;

use super::*;

#[derive(SystemParam)]
pub struct Spacecraft<'w, 's> {
    grids: Query<
        'w,
        's,
        (
            &'static SpacecraftGrid,
            &'static Children,
            &'static Transform,
        ),
    >,
    parts: Query<
        'w,
        's,
        (&'static PartInstance, &'static ChildOf, &'static Transform),
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

    pub fn part_transform(&self, grid: Entity) -> Result<Transform, QueryEntityError> {
        let (_, _, transform) = self.parts.get(grid)?;
        Ok(*transform)
    }

    pub fn compute_blueprint(&self, e: Entity) -> Result<Blueprint, QueryEntityError> {
        let mut blueprint = Blueprint::new();
        let (_, children, _) = self.grids.get(e)?;
        for c in children {
            let (part, _, _) = self.parts.get(*c)?;
            blueprint.add_part(part.name.clone(), part.placement, part.layer());
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
            if part.layer() != PartLayer::Internal {
                continue;
            }
            let offset = (coords.coord - part.origin()).inner();
            let dims = part.placement.part_aligned_dims().inner();
            if offset.x >= 0 && offset.y >= 0 && offset.x <= dims.x && offset.y <= dims.y {
                return Some(*id);
            }
        }

        None
    }

    pub fn cell_global_transform(&self, part: Entity, coord: PartCoord) -> Result<Transform, QueryEntityError> {
        let (part, parent, part_center) = self.parts.get(part)?;
        let (_, _, grid_transform) = self.grids.get(parent.0)?;
        let half_dims = part.placement.part_aligned_dims().to_meters() / 2.0;
        let offset = coord.to_meters() - half_dims + PartCoord::ONE.to_meters() / 2.0;
        let new_translation =
            part_center.translation + part_center.right() * offset.x + part_center.up() * offset.y;
        Ok(*grid_transform * part_center.with_translation(new_translation))
    }

    pub fn get_merge_transform(
        &self,
        host_part: Entity,
        target_part: Entity,
    ) -> Result<Transform, QueryEntityError> {
        let (port, parent, port_a) = self.parts.get(host_part)?;
        let (other_port, _, port_b) = self.parts.get(target_part)?;

        let (_, _, ownship_root) = self.grids.get(parent.0)?;

        let ownship_root = ownship_root.clone();

        let mut port_a = *port_a;
        let mut port_b = *port_b;

        let off_a = port.placement.part_aligned_dims().to_meters().x;
        let off_b = other_port.placement.part_aligned_dims().to_meters().x;

        port_a.translation += port_a.right() * off_a / 2.0;
        port_b.translation += port_b.right() * off_b / 2.0;

        Ok(target_docking_transform(ownship_root, port_a, port_b))
    }
}
