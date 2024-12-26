use crate::core::*;
use bevy::math::Vec2;

fn get_minimal_system(system: &OrbitalSystem, id: ObjectId) -> OrbitalSystem {
    let mut copy = system.clone();
    copy.objects.retain(|o| o.body.is_some() || o.id == id);
    copy
}
