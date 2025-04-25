use crate::planetary::GameState;
use crate::ui::InteractionEvent;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

pub fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut scroll: EventReader<MouseWheel>,
    mut events: EventWriter<InteractionEvent>,
) {
    state.input.set_buttons(keys.clone());

    for key in keys.get_just_pressed() {
        let e = match key {
            KeyCode::Period => InteractionEvent::SimFaster,
            KeyCode::Comma => InteractionEvent::SimSlower,
            KeyCode::Delete => InteractionEvent::Delete,
            KeyCode::KeyP => InteractionEvent::ToggleGraph,
            KeyCode::KeyG => InteractionEvent::CreateGroup,
            KeyCode::KeyC => InteractionEvent::ClearMissions,
            KeyCode::Enter => InteractionEvent::CommitMission,
            KeyCode::Minus => InteractionEvent::ZoomOut,
            KeyCode::Equal => InteractionEvent::ZoomIn,
            KeyCode::KeyR => InteractionEvent::Reset,
            KeyCode::KeyQ => InteractionEvent::ContextDependent,
            KeyCode::Tab => InteractionEvent::Orbits,
            KeyCode::Space => InteractionEvent::SimPause,
            KeyCode::Escape => InteractionEvent::ExitApp,
            KeyCode::KeyV => InteractionEvent::CursorMode,
            KeyCode::KeyM => InteractionEvent::DrawMode,
            KeyCode::KeyY => InteractionEvent::RedrawGui,
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

    let left_shift: bool = keys.pressed(KeyCode::ShiftLeft);

    for ev in scroll.read() {
        let e = match (ev.y > 0.0, left_shift) {
            (true, false) => InteractionEvent::ZoomIn,
            (false, false) => InteractionEvent::ZoomOut,
            (true, true) => InteractionEvent::SimFaster,
            (false, true) => InteractionEvent::SimSlower,
        };
        events.send(e);
    }
}
