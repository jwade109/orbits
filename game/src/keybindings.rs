use crate::ui::InteractionEvent;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

pub fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll: EventReader<MouseWheel>,
    mut events: EventWriter<InteractionEvent>,
) {
    for key in keys.get_just_pressed() {
        let e = match key {
            KeyCode::Period => InteractionEvent::SimFaster,
            KeyCode::Comma => InteractionEvent::SimSlower,
            KeyCode::Delete => InteractionEvent::Delete,
            KeyCode::KeyH => InteractionEvent::ToggleDebugMode,
            KeyCode::KeyF => InteractionEvent::Follow,
            KeyCode::Enter => InteractionEvent::CommitMission,
            KeyCode::Minus => InteractionEvent::ZoomOut,
            KeyCode::Equal => InteractionEvent::ZoomIn,
            KeyCode::KeyR => InteractionEvent::Reset,
            KeyCode::KeyQ => InteractionEvent::QueueOrbit,
            KeyCode::KeyC => InteractionEvent::ClearSelection,
            KeyCode::Tab => InteractionEvent::Orbits,
            KeyCode::Space => InteractionEvent::SimPause,
            KeyCode::Escape => InteractionEvent::ExitApp,
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
            KeyCode::ArrowUp => InteractionEvent::ThrustUp,
            KeyCode::ArrowDown => InteractionEvent::ThrustDown,
            KeyCode::ArrowLeft => InteractionEvent::ThrustLeft,
            KeyCode::ArrowRight => InteractionEvent::ThrustRight,
            KeyCode::KeyK => InteractionEvent::Spawn,
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
