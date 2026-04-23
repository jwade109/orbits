use crate::camera::Camera;
use crate::client::*;
use crate::utils::InputState;
use crate::multiplayer::*;
use crate::persistence::save_world;
use crate::sim::*;
use crate::sounds::*;
use crate::utils::*;
use bary_core::prelude::*;
use early_returns::*;
use log::*;
use rdev::Key;

pub fn command_selected_ships_to_waypoint(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
    p: Vec2,
    q: Vec2,
) {
    let free = some_or_return!(client.viewport.free());

    let mut successes = 0;

    let n_ships = free.selection_info.selected.len();

    for (i, loc) in free.selection_info.selected.iter().enumerate() {
        let s = if n_ships == 1 {
            1.0
        } else {
            i as f32 / (n_ships - 1) as f32
        };

        let waypont = p.lerp(q, s);

        let waypoint = Isometry2d::new(waypont, 0.0);

        if let Err(e) = set_primary_computer_waypoint(loc.grid_id, waypoint, world) {
            client.chat.log(format!("Failed to set waypoint: {e:?}"));
            continue;
        }

        if let Err(e) = set_primary_computer_state(loc.grid_id, true, world) {
            client
                .chat
                .log(format!("Failed to turn primary computer on: {e:?}"));
            continue;
        }

        successes += 1;
    }

    if successes == free.selection_info.selected.len() {
        sounds.push(SoundEffect::SetWaypoint);
    } else {
        sounds.push(SoundEffect::GenericFailure);
    }
}

pub fn explode_at_mouseover(world: &mut World, client: &mut ClientSpecificInfo) {
    let free = some_or_return!(client.viewport.free());
    let loc = some_or_return!(free.selection_info.hovered);
    explode_grid_at(loc, world);
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

pub fn destroy_top_layer_part_at_mouseover(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
) {
    let editor = some_or_return!(client.viewport.editor());
    let loc = some_or_return!(client.hovered_grid_loc());

    let result = if let Some(layer) = editor.layer {
        destroy_part_at_layer(world, loc, layer)
    } else {
        destroy_top_part_at(world, loc)
    };

    match result {
        Ok((instance, grid_id, grids)) => {
            info!("Removed part {:?}, grid {}", instance, grid_id);
            sounds.push(SoundEffect::DestroyPart);

            for grid_id in grids {
                let Ok(grid) = world.grids.try_get_mut(grid_id) else {
                    continue;
                };

                grid.velocity.translation += randvec(0.01, 0.03);
                grid.velocity.rotation += rand(-0.02, 0.02);
            }
        }
        Err(_e) => {
            // don't care.
        }
    }
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

pub fn panic_on_ctrl_d(input: &InputState) {
    if input.is_key_pressed(Key::ControlLeft) {
        info!("Exiting.");
        panic!();
    }
}

pub fn save_on_ctrl_s(world: &mut World, client: &mut ClientSpecificInfo) {
    let pressed_ctrl = client.input.is_key_pressed(Key::ControlLeft);

    if !pressed_ctrl {
        return;
    }

    let now = chrono::offset::Local::now();
    let filename = format!("./saves/world-{}/", now.format("%Y-%m-%d-%H-%M-%S"));
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

pub fn ping_on_alt_left_click(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    actions: &mut Vec<Action>,
    sounds: &mut SoundEffects,
) {
    let Some(screen_pos) = client.mouse_screen_position else {
        return;
    };

    if !client.input.is_key_pressed(Key::Alt) {
        return;
    }

    let pos = screen_to_world(&client.camera, screen_pos, client.screen_dims);

    let particle = PingParticle::new(pos);
    world.particles.push(particle);
    actions.push(Action::World(WorldAction::Ping(pos)));
    client.chat.log(format!("Pinged {}", pos));
    sounds.push(SoundEffect::Ping);
}

pub fn toggle_tracking_for_selected_grid(world: &mut World, client: &mut ClientSpecificInfo) {
    let free = some_or_return!(client.viewport.free());
    let grid_id = some_or_return!(free.selection_info.first_selected_grid());

    match toggle_tracking(world, grid_id) {
        Ok(true) => client.chat.log(format!("Enabled tracking for {}", grid_id)),
        Ok(false) => client
            .chat
            .log(format!("Disabled tracking for {}", grid_id)),
        Err(e) => client
            .chat
            .log(format!("Failed to toggle tracking: {:?}", e)),
    }
}

pub fn destroy_selected_parts(_world: &mut World, client: &mut ClientSpecificInfo) {
    let free = some_or_return!(client.viewport.free());
    let s = format!("TODO DESTROY PARTS: {:?}", free.selection_info.selected);
    client.chat.log(s);
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

pub fn spawn_random_ship_on_p(world: &mut World) {
    if let Ok(grid_id) = spawn_grid_with_random_name(world, "remora") {
        let pos = randvec(10.0, 200.0);
        _ = set_grid_pose(world, grid_id, Isometry2d::from_pos(pos));
    }
}

pub fn update_center_of_mass_on_m(world: &mut World) {
    for grid in world.grids.values_mut() {
        _ = update_grid_physical_props(grid, &mut world.parts);
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
