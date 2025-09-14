use crate::game::GameState;
use crate::sim_rate::SimRate;
use crate::ui::InteractionEvent;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

pub fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    scroll: EventReader<MouseWheel>,
    mut events: EventWriter<InteractionEvent>,
) {
    state.input.set_buttons(keys.clone());
    state.input.set_scroll(scroll);

    let ctrl = keys.pressed(KeyCode::ControlLeft);
    let shift = keys.pressed(KeyCode::ShiftLeft);

    for key in keys.get_just_pressed() {
        let e = match (ctrl, shift, key) {
            (_, _, KeyCode::Period) => InteractionEvent::SimFaster,
            (_, _, KeyCode::Comma) => InteractionEvent::SimSlower,
            (_, _, KeyCode::Slash) => InteractionEvent::SetSim(SimRate::RealTime),
            (_, _, KeyCode::Delete) => InteractionEvent::Delete,
            (_, _, KeyCode::KeyG) => InteractionEvent::CreateGroup,
            (_, _, KeyCode::Minus) => InteractionEvent::ZoomOut,
            (_, _, KeyCode::Equal) => InteractionEvent::ZoomIn,
            (_, _, KeyCode::KeyR) => InteractionEvent::Reset,
            (_, _, KeyCode::Space) => InteractionEvent::SimPause,
            (_, _, KeyCode::Escape) => InteractionEvent::Escape,
            (_, _, KeyCode::KeyV) => InteractionEvent::CursorMode,
            (_, _, KeyCode::F11) => InteractionEvent::ToggleFullscreen,
            (_, _, KeyCode::Backquote) => InteractionEvent::ToggleDebugConsole,
            _ => continue,
        };

        events.send(e);
    }
}
