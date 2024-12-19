use crate::core::*;
use bevy::math::Vec2;
use std::time::Duration;

fn get_minimal_system(system: &OrbitalSystem, id: ObjectId) -> OrbitalSystem {
    let mut copy = system.clone();
    copy.objects.retain(|o| o.body.is_some() || o.id == id);
    copy
}

pub fn get_future_positions(
    system: &OrbitalSystem,
    id: ObjectId,
    steps: usize,
) -> (Vec<Vec2>, bool) {
    let mut minimal = get_minimal_system(system, id);
    let positions: Vec<_> = (0..steps)
        .filter_map(|_| {
            minimal.step();
            Some(minimal.transform_from_id(Some(id))?.pos)
        })
        .collect();
    let abridged = positions.len() < steps;
    (positions, abridged)
}

pub struct Controller {
    pub target: ObjectId,
    pub accel: Vec2
}
