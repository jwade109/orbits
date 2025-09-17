use crate::prelude::*;
use bevy::input::keyboard::KeyCode;
use starling::prelude::*;

fn combo_just_pressed(input: &InputState, keys: &[KeyCode]) -> bool {
    if let Some(l) = keys.last() {
        keys.iter().all(|k| input.is_pressed(*k)) && input.just_pressed(*l)
    } else {
        false
    }
}

pub fn on_global_render_tick(state: &mut GameState) -> bool {
    let mut take = Take::from_opt(state.input.position(MouseButt::Hover, FrameId::Current));

    if state.input.just_pressed(KeyCode::KeyK) {
        if let Some(t) = &mut state.tutorial {
            t.prev();
        }
    }

    if state.input.just_pressed(KeyCode::KeyL) {
        if let Some(t) = &mut state.tutorial {
            let force = state.input.is_pressed(KeyCode::ControlLeft);
            t.next(force);
        }
    }

    if state
        .input
        .on_frame(MouseButt::Right, FrameId::Down)
        .is_some()
        && state.input.is_pressed(KeyCode::ShiftLeft)
    {
        if let Some(p) = take.take() {
            state.spawn_window(WindowClass::Hello, p);
            return true;
        }
    }

    state.windows.sort_by_key(|w| w.is_focused as u8);

    let mut events = Vec::new();

    for button in &mut state.buttons {
        if let Some(e) = button.on_mouse_move(&mut take) {
            events.push(e);
        }
    }

    for window in state.windows.iter_mut().rev() {
        window.on_mouse_move(&mut take);

        for e in &state.input.keyboard_events {
            if let Some(e) = window.on_key(e) {
                events.push(e);
            }
        }
    }

    if state.input.just_pressed(KeyCode::KeyH) {
        state.reset_camera();
    }

    if state.input.just_pressed(KeyCode::KeyV) {
        state.zoom_to_vehicle(true);
    }

    if state.input.just_pressed(KeyCode::KeyY) {
        state.arrange_windows(false);
    }

    if state.input.just_pressed(KeyCode::KeyJ) {
        state.arrange_windows(true);
    }

    if state.console.is_active() {
        if let Some((decl, args)) = state.console.process_input(&mut state.input) {
            decl.execute(state, args);
        }
        return true;
    }

    if let Some(_) = state.input.on_frame(MouseButt::Left, FrameId::Down) {
        for button in &mut state.buttons {
            button.on_left_mouse_down();
        }
        for window in &mut state.windows {
            if let Some(e) = window.on_left_mouse_down() {
                events.push(e);
            }
        }
    }

    if let Some(_) = state.input.on_frame(MouseButt::Left, FrameId::Up) {
        for button in &mut state.buttons {
            if let Some(e) = button.on_left_mouse_up() {
                events.push(e);
            }
        }
        for window in &mut state.windows {
            window.on_left_mouse_up();
        }
    }

    if !events.is_empty() {
        for e in events {
            state.on_button_event(e);
        }
        return true;
    }

    if state.input.just_pressed(KeyCode::KeyB) {
        state.spawn_new();
    }

    if state.input.just_pressed(KeyCode::Delete) {
        if let Some(p) = state.piloting() {
            state.delete_orbiter(p);
        }
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
