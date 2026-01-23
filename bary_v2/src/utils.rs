use bevy::prelude::*;

pub fn swallow_optional(In(_): In<Option<()>>) {}

pub fn swallow_result(In(_): In<Result>) {}
