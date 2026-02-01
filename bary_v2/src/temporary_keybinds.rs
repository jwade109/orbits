use bary_core::prelude::GridPlacement;
use bary_core::prelude::*;
use bevy::prelude::*;
use early_returns::some_or_return;

use crate::{
    AddHose, CursorWorldPosition, GridPlacementEffect, SelectedSpacecraft, Settings,
    SpacecraftEvent,
};

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
    let start = (a.part.entity, a.part.coord);
    let end = (b.part.entity, b.part.coord);

    commands.trigger(AddHose { start, end });
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

pub fn spawn_grid_effect_on_p(
    mut commands: Commands,
    sel: Res<SelectedSpacecraft>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.just_pressed(KeyCode::KeyP) {
        return;
    }

    let grid = some_or_return!(sel.primary).grid.entity;

    let x = randint(-20, 20);
    let y = randint(-20, 20);

    let placement = GridPlacement::new((x, y), Rotation::East, (5, 3));

    let effect = GridPlacementEffect::new(grid, placement);

    commands.spawn(effect);
}
