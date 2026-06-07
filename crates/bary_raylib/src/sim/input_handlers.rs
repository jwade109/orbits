use crate::camera::Camera;
use crate::editor_state::EditorState;
use crate::sim::*;
use crate::sounds::*;
use bary_core::prelude::*;
use bary_input::*;
use bary_sim::*;
use early_returns::*;
use log::*;

pub fn drive_ship_on_enter(client: &mut ClientSpecificInfo, world: &World) -> Option<WorldDelta> {
    let player_id = client.player_id?;
    let free = client.viewport.free()?;
    let grid_id = free.selection_info.first_selected_grid()?;
    let player = world.players.try_get(player_id).ok()?;

    if player.is_driving() {
        Some(WorldDelta::PlayerPilotingExitGrid(player_id))
    } else {
        Some(WorldDelta::PlayerPilotingEnterGrid { player_id, grid_id })
    }
}

pub fn command_selected_ships_to_waypoint(
    client: &mut ClientSpecificInfo,
    p: Vec2,
    q: Vec2,
) -> Vec<WorldDelta> {
    let mut deltas = Vec::new();

    let Some(free) = client.viewport.free() else {
        return deltas;
    };

    let n_ships = free.selection_info.selected.len();

    for (i, loc) in free.selection_info.selected.iter().enumerate() {
        let s = if n_ships == 1 {
            1.0
        } else {
            i as f32 / (n_ships - 1) as f32
        };

        let waypont = p.lerp(q, s);
        let rotation = Vec2::X.angle_to(q - p);
        let waypoint = Isometry2d::new(waypont, rotation);

        let delta = WorldDelta::SetWaypoint {
            grid_id: loc.grid_id,
            waypoint,
        };

        deltas.push(delta);
    }

    deltas
}

pub fn editor_copy_on_control_c(world: &World, client: &mut ClientSpecificInfo) {
    if !client.input.is_key_pressed(Key::ControlLeft) {
        return;
    }

    let editor = some_or_return!(client.viewport.editor());
    let grid = some_or_return!(world.grids.get(editor.vehicle));

    let s = format!("TODO Ctrl-C behavior: {}", grid.name);

    client.chat.log(s);
}

pub fn apply_scroll_wheel_to_camera_target(delta_y: i64, target: &mut Camera) {
    let scale = 1.15;
    if delta_y > 0 {
        target.zoom *= scale;
    } else if delta_y < 0 {
        target.zoom /= scale;
    }
}

pub fn editor_layer_shift_on_page_key(client: &mut ClientSpecificInfo, is_up: bool) {
    let Viewport::Editor(editor) = &mut client.viewport else {
        return;
    };

    editor.layer = if is_up {
        enum_iterator::next_cycle(&editor.layer)
    } else {
        enum_iterator::previous_cycle(&editor.layer)
    };
}

pub fn save_on_ctrl_s(world: &World, client: &mut ClientSpecificInfo) {
    let pressed_ctrl = client.input.is_key_pressed(Key::ControlLeft);

    if !pressed_ctrl {
        return;
    }

    let now = chrono::offset::Local::now();

    let home = std::env::var("HOME").unwrap();

    let filename = format!(
        "{}/.barycenter/saves/world-{}/",
        home,
        now.format("%Y-%m-%d-%H-%M-%S")
    );
    match save_world(&filename, world, true) {
        Ok(_) => {
            let s = format!("Saved to {}", filename);
            client.chat.log(s);
        }
        Err(e) => {
            let s = format!("Failed to save: {:?}", e);
            client.chat.log(s);
        }
    }
}

pub fn toggle_following_on_key_f(client: &mut ClientSpecificInfo, sounds: &mut SoundEffects) {
    let free = some_or_return!(client.viewport.free());
    let grid_id = some_or_return!(free.selection_info.first_selected_grid());

    if client.viewport.look_at(grid_id) {
        sounds.push(SoundEffect::Follow);
        debug!("Following {}", grid_id);
    }
}

pub fn reset_camera_on_ctrl_r(client: &mut ClientSpecificInfo) {
    if client.input.is_key_pressed(Key::ControlLeft) {
        debug!("Reset camera");
        client.target_camera.isometry.translation = Vec2::ZERO;
        client.target_camera.isometry.rotation = 0.0;
        client.target_camera.zoom = 8.0;
    }
}

pub fn lock_rotation_on_key_r(client: &mut ClientSpecificInfo) {
    if client.input.is_key_pressed(Key::ControlLeft) {
        return;
    }
    if let Viewport::Free(fly) = &mut client.viewport {
        debug!("Toggle lock rotation");
        fly.lock_rotation ^= true;
    }
}

pub fn rotate_editor_part_on_key_r(client: &mut ClientSpecificInfo) {
    if client.input.is_key_pressed(Key::ControlLeft) {
        return;
    }
    if let Viewport::Editor(editor) = &mut client.viewport {
        if editor.prototype_id.is_some() {
            editor.part_rotation = editor.part_rotation.next();
        } else {
            editor.camera_rotation = editor.camera_rotation.next();
        }
    }
}

pub fn leave_ship_editor_on_escape(client: &mut ClientSpecificInfo, sounds: &mut SoundEffects) {
    let Viewport::Editor(editor) = &client.viewport else {
        return;
    };

    client.viewport = Viewport::Free(FreeFlying {
        follow_vehicle: Some(editor.vehicle),
        lock_rotation: false,
        selection_info: SelectionInfo::selecting(editor.vehicle),
        waypoint_widget: None,
        hovered_chunk: None,
        offset: PartCoord::ZERO,
        rotation: Rotation::East,
    });

    client.target_camera.zoom = 20.0;
    client.target_camera.isometry.rotation = 0.0;

    client.chat.log("Left ship editor");
    sounds.push(SoundEffect::LeaveEditor);
}

pub fn enter_ship_editor(
    world: &World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
) {
    let free = some_or_return!(client.viewport.free());
    let grid_id = some_or_return!(free.selection_info.first_selected_grid());
    let grid = ok_or_return!(world.grids.try_get(grid_id));

    let centroid = grid.centroid();

    client.viewport = Viewport::Editor(EditorState {
        vehicle: grid_id,
        target_offset: centroid,
        actual_offset: centroid,
        camera_rotation: Rotation::East,
        prototype_id: None,
        part_rotation: Rotation::East,
        layer: Some(PartLayer::Internal),
        select_start: None,
        hovered: None,
    });

    client.target_camera.zoom = client.target_camera.zoom.max(40.0);

    client.chat.log("Switched to ship editor");
    sounds.push(SoundEffect::OpenEditor);
}

pub fn pipette_part_if_in_editor_on_q(world: &World, client: &mut ClientSpecificInfo) {
    let editor = some_or_return!(client.viewport.editor_mut());

    if editor.prototype_id.is_some() {
        editor.prototype_id = None;
        return;
    }
    editor.prototype_id = None;

    let coord = some_or_return!(editor.hovered);

    let grid = ok_or_return!(world.grids.try_get(editor.vehicle));
    let Some(occ) = grid.get_parts_at(coord) else {
        editor.layer = None;
        return;
    };

    // use the focus layer to pipette if it's available; otherwise, use the top one
    let part_id = if let Some(layer) = editor.layer {
        occ.at_layer(layer)
    } else {
        occ.top()
    };

    let part_id = some_or_return!(part_id);
    let part = ok_or_return!(world.parts.try_get(part_id));

    editor.prototype_id = Some(part.prototype);
    editor.part_rotation = part.region.rot();
    editor.layer = Some(part.layer);
}
