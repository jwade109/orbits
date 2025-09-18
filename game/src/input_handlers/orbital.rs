use crate::prelude::*;
use bevy::input::keyboard::KeyCode;
use starling::prelude::*;

pub fn on_orbital_render_tick(state: &mut GameState) {

    state
        .orbital_context
        .camera
        .handle_input(&state.input, &state.settings);

    if state.input.just_pressed(KeyCode::Slash) {
        state.universe_ticks_per_game_tick = SimRate::RealTime;
        return;
    }

    if state.input.just_pressed(KeyCode::Comma) {
        state.sim_slower();
        return;
    }

    if state.input.just_pressed(KeyCode::Period) {
        state.sim_faster();
        return;
    }

    state.orbital_context.hovered_entity =
        if let Some(p) = state.input.position(MouseButt::Hover, FrameId::Current) {
            let dist = (SPACECRAFT_HOVER_RADIUS / state.orbital_context.scale()).max(0.0);
            let w = state.orbital_context.c2w(p);
            nearest_orbiter_or_planet(&state.universe, w, dist)
        } else {
            None
        };

    if let Some(_) = state.input.on_frame(MouseButt::Left, FrameId::Down) {
        if state.input.is_pressed(KeyCode::ControlLeft) {
            state.orbital_context.following = state.orbital_context.hovered_entity;
            state.orbital_context.camera.clear_offset();
        } else {
            if let Some(h) = state.orbital_context.hovered_entity {
                state.orbital_context.piloting = Some(h);
                state.sounds.play_once("soft-pulse-higher.ogg", 0.3);
            } else {
                state.orbital_context.piloting = None;
                state.sounds.play_once("soft-pulse.ogg", 0.3);
            }
        }
    }

    if let Some(_) = state.input.on_frame(MouseButt::Right, FrameId::Down) {
        || -> Option<()> {
            let pilot = state.orbital_context.piloting?;
            let sv = state.universe.spacecraft.get_mut(&pilot)?;
            if state.orbital_context.hovered_entity != Some(pilot) {
                if sv.target() == state.orbital_context.hovered_entity {
                    sv.set_target(None);
                } else {
                    sv.set_target(state.orbital_context.hovered_entity);
                }
            }
            Some(())
        }();
    }
}
