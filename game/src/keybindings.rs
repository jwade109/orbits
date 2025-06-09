use crate::game::GameState;
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

    for key in keys.get_just_pressed() {
        let e = match key {
            KeyCode::Period => InteractionEvent::SimFaster,
            KeyCode::Comma => InteractionEvent::SimSlower,
            KeyCode::Slash => InteractionEvent::SetSim(0),
            KeyCode::Delete => InteractionEvent::Delete,
            KeyCode::KeyG => InteractionEvent::CreateGroup,
            KeyCode::KeyC => InteractionEvent::ClearMissions,
            KeyCode::Enter => InteractionEvent::CommitMission,
            KeyCode::Minus => InteractionEvent::ZoomOut,
            KeyCode::Equal => InteractionEvent::ZoomIn,
            KeyCode::KeyR => InteractionEvent::Reset,
            KeyCode::KeyQ => InteractionEvent::ContextDependent,
            KeyCode::Tab => InteractionEvent::Orbits,
            KeyCode::Space => InteractionEvent::SimPause,
            KeyCode::Escape => InteractionEvent::Escape,
            KeyCode::KeyV => InteractionEvent::CursorMode,
            KeyCode::KeyM => InteractionEvent::DrawMode,
            KeyCode::F11 => InteractionEvent::ToggleFullscreen,
            _ => continue,
        };

        events.send(e);
    }

    for key in keys.get_pressed() {
        let e = match (keys.pressed(KeyCode::ControlLeft), key) {
            (_, KeyCode::KeyK) => InteractionEvent::Spawn,
            (_, KeyCode::ArrowUp) => InteractionEvent::Thrust(1),
            (_, KeyCode::ArrowDown) => InteractionEvent::Thrust(-1),
            (false, KeyCode::ArrowLeft) => InteractionEvent::TurnLeft,
            (false, KeyCode::ArrowRight) => InteractionEvent::TurnRight,
            (true, KeyCode::ArrowLeft) => InteractionEvent::StrafeLeft,
            (true, KeyCode::ArrowRight) => InteractionEvent::StrafeRight,
            _ => continue,
        };

        events.send(e);
    }
}
