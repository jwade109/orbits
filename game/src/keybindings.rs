use crate::planetary::GameState;
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
        let e = match key {
            KeyCode::KeyW => InteractionEvent::MoveUp,
            KeyCode::KeyA => InteractionEvent::MoveLeft,
            KeyCode::KeyS => InteractionEvent::MoveDown,
            KeyCode::KeyD => InteractionEvent::MoveRight,
            KeyCode::KeyK => InteractionEvent::Spawn,
            KeyCode::ArrowUp => InteractionEvent::ThrustForward,
            KeyCode::ArrowLeft => InteractionEvent::TurnLeft,
            KeyCode::ArrowRight => InteractionEvent::TurnRight,
            _ => continue,
        };

        events.send(e);
    }
}
