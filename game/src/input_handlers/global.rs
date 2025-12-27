use crate::prelude::*;
use crate::starling::prelude::*;
use bevy::input::keyboard::KeyCode;

fn combo_just_pressed(input: &InputState, keys: &[KeyCode]) -> bool {
    if let Some(l) = keys.last() {
        keys.iter().all(|k| input.is_pressed(*k)) && input.just_pressed(*l)
    } else {
        false
    }
}

pub fn on_global_render_tick(state: &mut GameState) -> bool {
    let mut take = Take::from_opt(state.input.position(MouseButt::Hover, FrameId::Current));

    if state.input.just_pressed(KeyCode::Escape) {
        state.shutdown_with_prompt();
        return true;
    }

    if state.input.just_pressed(KeyCode::KeyH) {
        state.reset_camera();
    }

    if state.input.just_pressed(KeyCode::KeyV) {
        state.zoom_to_vehicle(true);
    }

    if combo_just_pressed(
        &state.input,
        &[KeyCode::ControlLeft, KeyCode::ShiftLeft, KeyCode::KeyT],
    ) {
        state.settings.draw_transform_tree = !state.settings.draw_transform_tree;
        if state.settings.draw_transform_tree {
            state.notice("Transform tree drawn");
        } else {
            state.notice("Transform tree hidden");
        }
        return true;
    }

    if combo_just_pressed(
        &state.input,
        &[KeyCode::ControlLeft, KeyCode::ShiftLeft, KeyCode::KeyM],
    ) {
        state.settings.music_muted = !state.settings.music_muted;
        if state.settings.music_muted {
            state.notice("Music muted");
        } else {
            state.notice("Music unmuted")
        }
        return true;
    }

    if combo_just_pressed(
        &state.input,
        &[KeyCode::ControlLeft, KeyCode::ShiftLeft, KeyCode::KeyP],
    ) {
        state.settings.draw_thrust_particles = !state.settings.draw_thrust_particles;
        if state.settings.draw_thrust_particles {
            state.notice("Enabled thrust particles");
        } else {
            state.notice("Disabled thrust particles")
        }
        return true;
    }

    if combo_just_pressed(
        &state.input,
        &[KeyCode::ControlLeft, KeyCode::ShiftLeft, KeyCode::KeyD],
    ) {
        state.settings.show_debug_info = !state.settings.show_debug_info;
        return true;
    }

    if state.input.is_pressed(KeyCode::ShiftLeft) && state.input.is_pressed(KeyCode::ControlLeft) {
        let delta = if state.input.just_pressed(KeyCode::Minus) {
            Some(-1.0)
        } else if state.input.just_pressed(KeyCode::Equal) {
            Some(1.0)
        } else {
            None
        };

        if let Some(delta) = delta {
            state.settings.ui_button_height =
                (state.settings.ui_button_height + delta).clamp(3.0, 40.0);
            state.notice(format!("UI scale: {}", state.settings.ui_button_height));
        }
    }

    state.handle_click_events();

    state.is_hovering_over_ui() || take.take().is_none()
}
