use bary_core::prelude::*;
use bevy::prelude::*;
use early_returns::some_or_return;

use crate::{AddHose, CursorWorldPosition, SelectedSpacecraft, Settings, SpacecraftEvent};

pub fn add_hose_on_h(
    mut commands: Commands,
    sel: Res<SelectedSpacecraft>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.just_pressed(KeyCode::KeyH) {
        return;
    }

    let a = some_or_return!(sel.primary);
    let b = some_or_return!(sel.secondary);
    commands.trigger(AddHose { a, b });
}

pub fn toggle_inv_on_alt(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<Settings>) {
    if keys.just_pressed(KeyCode::AltLeft) {
        settings.draw_inventories = !settings.draw_inventories;
    }
}

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
