use bary_core::prelude::*;
use bevy::prelude::*;

use crate::{CursorWorldPosition, SpacecraftEvent};

pub fn spawn_random_ship_on_y(
    mut commands: Commands,
    pos: Res<CursorWorldPosition>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.just_pressed(KeyCode::KeyY) {
        return;
    }

    let Some(pos) = pos.get() else {
        return;
    };

    let angle = rand(0.0, 2.0 * PI);

    commands.trigger(SpacecraftEvent::SpawnVehicle {
        ship_name: "Whatever".into(),
        blueprint_name: "remora".into(),
        pos,
        angle,
    });
}
