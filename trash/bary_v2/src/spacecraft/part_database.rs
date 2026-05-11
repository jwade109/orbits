use bevy::prelude::*;

#[derive(Resource, Deref)]
pub struct PartsResource(pub bary_core::prelude::PartDatabase);
