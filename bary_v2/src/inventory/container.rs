use bary_core::prelude::{PartCoord, transform_to_isometry};
use bevy::prelude::*;
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
