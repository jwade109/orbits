use bary_core::prelude::GridPlacement;
use bary_core::prelude::*;
use bevy::prelude::*;
use early_returns::some_or_return;

use crate::{
    AddHose, AddPipe, CursorWorldPosition, GridPlacementEffect, SelectedSpacecraft, Settings,
    SpacecraftEvent,
};

pub fn add_hose_or_pipe_on_h_or_p(
    mut commands: Commands,
    sel: Res<SelectedSpacecraft>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let make_hose = keys.just_pressed(KeyCode::KeyH);
    let make_pipe = keys.just_pressed(KeyCode::KeyP);

    if !make_hose && !make_pipe {
        return;
    }

    let a = some_or_return!(sel.primary);
    let b = some_or_return!(sel.secondary);
    let start = (a.part.entity, a.part.coord);
    let end = (b.part.entity, b.part.coord);

    if make_hose {
        commands.trigger(AddHose { start, end });
    } else if make_pipe {
        commands.trigger(AddPipe { start, end });
    }
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

pub fn spawn_grid_effect_on_r(
    mut commands: Commands,
    sel: Res<SelectedSpacecraft>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }

    let grid = some_or_return!(sel.primary).grid.entity;

    let x = randint(-20, 20);
    let y = randint(-20, 20);

    let placement = GridPlacement::new((x, y), Rotation::East, (5, 3));

    let effect = GridPlacementEffect::new(grid, placement);

    commands.spawn(effect);
}
