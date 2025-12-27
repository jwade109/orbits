use crate::prelude::*;
use bevy::input::keyboard::KeyCode;
use crate::starling::prelude::*;

pub fn on_editor_render_tick(state: &mut GameState) {
    state
        .editor_context
        .camera
        .handle_input(&state.input, &state.settings);

    if state.is_hovering_over_ui() {
        return;
    }

    if let Some(p) = state.input.on_frame(MouseButt::Left, FrameId::Down) {
        let p = state.editor_context.c2w(p);
        if let Some((id, _)) = state.editor_context.get_part_at(graphics_cast(p)) {
            state.editor_context.selected_part = Some(id)
        } else {
            state.editor_context.selected_part = None;
        }
    }

    if state.input.is_pressed(KeyCode::ShiftLeft) {
        if let Some((pos, proto)) = Editor::current_part_and_cursor_position(state) {
            if state.editor_context.snap_info.is_none() {
                let rot = state.editor_context.rotation;
                let dims = pixel_dims_with_rotation(rot, &proto);
                state.editor_context.snap_info = Some((pos, dims));
            }
        } else {
            state.editor_context.snap_info = None;
        }
    } else {
        state.editor_context.snap_info = None;
    }

    if let Some(_) = state.input.position(MouseButt::Left, FrameId::Current) {
        if let Some((p, part)) = Editor::current_part_and_cursor_position(state) {
            state.editor_context.try_place_part(p, part);
        }
    } else if let Some(p) = state.input.on_frame(MouseButt::Right, FrameId::Down) {
        state
            .editor_context
            .remove_part_at(graphics_cast(state.editor_context.c2w(p)));
    } else if state.input.just_pressed(KeyCode::KeyQ) {
        if state.editor_context.cursor_state.current_part().is_some() {
            state.editor_context.cursor_state = CursorState::None;
        } else if let Some(p) = state.input.position(MouseButt::Hover, FrameId::Current) {
            if let Some((_, instance)) = state
                .editor_context
                .get_part_at(graphics_cast(state.editor_context.c2w(p)))
            {
                let instance = instance.clone();
                state.editor_context.rotation = instance.rotation();
                state.editor_context.cursor_state = CursorState::Part(instance.prototype().clone());
            } else {
                state.editor_context.cursor_state = CursorState::None;
            }
        }
    }

    if state.input.just_pressed(KeyCode::KeyR) {
        state.editor_context.rotation = enum_iterator::next_cycle(&state.editor_context.rotation);
    }

    if state.input.is_pressed(KeyCode::ControlLeft) && state.input.just_pressed(KeyCode::KeyZ) {
        state.editor_context.undo();
    }

    if state.input.just_pressed(KeyCode::KeyO) {
        state.editor_context.atmo += 1;
    }

    if state.input.just_pressed(KeyCode::KeyL) {
        state.editor_context.atmo -= 1;
    }

    state.editor_context.atmo = state.editor_context.atmo.clamp(0, 10);
}
