use bevy::prelude::*;

#[derive(Component, Deref)]
#[relationship_target(relationship = PartInGrid, linked_spawn)]
pub struct GridParts(Vec<Entity>);

#[derive(Component, Deref)]
#[relationship(relationship_target = GridParts)]
pub struct PartInGrid(pub Entity);
