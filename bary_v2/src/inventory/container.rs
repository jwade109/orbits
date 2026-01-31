use bary_core::prelude::{PartCoord, transform_to_isometry};
use bevy::{color::palettes::tailwind::BLUE_500, prelude::*};
use early_returns::ok_or_continue;

use crate::{CellPosition, Spacecraft};

#[derive(Component, Deref)]
#[relationship_target(relationship = ContainerInPart, linked_spawn)]
pub struct PartContainers(Vec<Entity>);

#[derive(Component, Deref)]
#[relationship(relationship_target = PartContainers)]
pub struct ContainerInPart(pub Entity);

#[derive(Component, Debug, Clone, Copy)]
pub struct ContainerLocation {
    pub origin: PartCoord,
    pub dims: PartCoord,
}

impl ContainerLocation {
    pub fn contains(&self, pos: PartCoord) -> bool {
        let off = (pos - self.origin).inner();
        let dims = self.dims.inner();
        off.x >= 0 && off.x < dims.x && off.y >= 0 && off.y < dims.y
    }
}

pub fn debug_draw_inventory_vessels(
    mut gizmos: Gizmos,
    containers: Query<(&ContainerLocation, &ContainerInPart)>,
    spacecraft: Spacecraft,
) {
    for (loc, part) in containers {
        let origin = ok_or_continue!(spacecraft.cell_global_transform(
            **part,
            loc.origin,
            CellPosition::BottomLeft
        ));
        let opposite = ok_or_continue!(spacecraft.cell_global_transform(
            **part,
            loc.origin + loc.dims,
            CellPosition::BottomLeft
        ));

        let center = origin.translation * 0.5 + opposite.translation * 0.5;

        let isometry = transform_to_isometry(origin.with_translation(center));

        let size = loc.dims.to_meters();

        gizmos.rect_2d(isometry, size, BLUE_500);
    }
}
