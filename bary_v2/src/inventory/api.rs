use bary_core::prelude::PartCoord;
use bevy::prelude::*;
use bevy_ecs::{query::QueryEntityError, system::SystemParam};

use crate::{ContainerInPart, ContainerLocation, PartContainers};

#[derive(Debug)]
pub enum FailedLookup {
    QueryError(QueryEntityError),
    NoResult,
}

impl From<QueryEntityError> for FailedLookup {
    fn from(value: QueryEntityError) -> Self {
        Self::QueryError(value)
    }
}

#[derive(SystemParam)]
pub struct InventoryApi<'w, 's> {
    parts: Query<'w, 's, &'static PartContainers>,
    containers: Query<'w, 's, &'static ContainerLocation>,
}

impl<'w, 's> InventoryApi<'w, 's> {
    pub fn get_container(&self, container: Entity) -> Result<&ContainerLocation, QueryEntityError> {
        self.containers.get(container)
    }

    pub fn find_container_at(&self, part: Entity, pos: PartCoord) -> Result<Entity, FailedLookup> {
        let children = self.parts.get(part)?;
        for child in children.iter() {
            let loc = self.containers.get(child)?;
            if loc.contains(pos) {
                info!("Yay! {:?}", loc);
                return Ok(child);
            }
        }
        Err(FailedLookup::NoResult)
    }
}
