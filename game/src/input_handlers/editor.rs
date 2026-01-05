use crate::prelude::*;
use crate::starling::prelude::*;
use bevy::input::keyboard::KeyCode;

pub fn on_editor_render_tick(state: &mut GameState) {
    state
        .editor_context
        .camera
        .handle_input(&state.input, &state.settings);

    if state.input.is_pressed(KeyCode::ControlLeft) && state.input.just_pressed(KeyCode::KeyC) {
        Editor::on_ctrl_c(state);
    }

    if let Some(p) = state.input.on_frame(MouseButt::Left, FrameId::Down) {
        let is_shift = state.input.is_pressed(KeyCode::ShiftLeft);
        Editor::on_left_click_down(state, p, is_shift);
    }

    if let Some(p) = state.input.on_frame(MouseButt::Left, FrameId::Current) {
        Editor::on_left_click_held(state, p);
    }

    if let Some(_) = state.input.on_frame(MouseButt::Left, FrameId::Up) {
        Editor::on_left_click_release(state);
    }

    Editor::process_holding_shift(state);

    if let Some(_) = state.input.position(MouseButt::Left, FrameId::Current) {
        // place a single part
        if let Some((p, part)) = Editor::current_part_and_cursor_position(state) {
            state
                .editor_context
                .try_place_part(p, part, state.editor_context.rotation);
        }

        if let Some(bp) = state.editor_context.cursor_state.blueprint() {
            if let Some(pos) = state.input.on_frame(MouseButt::Left, FrameId::Down) {
                let pos = PartCoord::from_meters_floored(state.editor_context.c2w(pos));
                let bp = bp.clone();
                for (_, part) in bp.parts() {
                    let proto = part.proto.clone();
                    state
                        .editor_context
                        .try_place_part(pos + part.pos, proto, part.rot);
                }
                for (_, part) in bp.pipes() {
                    state.editor_context.blueprint.add_pipe(part.with_offset(pos));
                }
            }
        }
    }

    if let Some(p) = state.input.on_frame(MouseButt::Right, FrameId::Down) {
        Editor::on_right_click_down(state, p);
    }

    if state.input.just_pressed(KeyCode::KeyQ) {
        Editor::on_press_q(state);
    }

    if state.input.just_pressed(KeyCode::KeyR) {
        Editor::on_press_r(state);
    }

    if state.input.is_pressed(KeyCode::ControlLeft) && state.input.just_pressed(KeyCode::KeyZ) {
        state.editor_context.undo();
    }
}
