use bary_core::prelude::*;
use bary_orbital::VehicleControl;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PlayerState {
    Flying(Isometry2d),
    PilotingGrid(Ent, VehicleControl),
}

impl std::fmt::Display for PlayerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Flying(_) => write!(f, "Flying around"),
            Self::PilotingGrid(id, _) => write!(f, "Piloting {}", id),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Player {
    pub name: String,
    pub cursor_world_position: Option<Vec2>,
    pub state: PlayerState,
}

impl Player {
    pub fn driving_grid(&self) -> Option<Ent> {
        match self.state {
            PlayerState::PilotingGrid(id, _) => Some(id),
            _ => None,
        }
    }

    pub fn set_position(&mut self, iso: Isometry2d) {
        self.state = PlayerState::Flying(iso);
    }

    pub fn is_driving(&self) -> bool {
        matches!(self.state, PlayerState::PilotingGrid(_, _))
    }

    pub fn is_flying(&self) -> bool {
        matches!(self.state, PlayerState::Flying(_))
    }
}

// fn keyboard_control_law(keys: &ButtonInput<KeyCode>) -> VehicleControl {
//     let mut ctrl = VehicleControl::NULLOPT;

//     let docking_mode = keys.pressed(KeyCode::ControlLeft);

//     if docking_mode {
//         ctrl.plus_x.throttle = keys.pressed(KeyCode::ArrowUp) as u8 as f32;
//         ctrl.plus_y.throttle = keys.pressed(KeyCode::ArrowRight) as u8 as f32;
//         ctrl.neg_x.throttle = keys.pressed(KeyCode::ArrowDown) as u8 as f32;
//         ctrl.neg_y.throttle = keys.pressed(KeyCode::ArrowLeft) as u8 as f32;
//     } else {
//         ctrl.plus_x.throttle = keys.pressed(KeyCode::ArrowUp) as u8 as f32;
//         ctrl.neg_x.throttle = keys.pressed(KeyCode::ArrowDown) as u8 as f32;

//         ctrl.attitude = if keys.pressed(KeyCode::ArrowLeft) {
//             10.0
//         } else if keys.pressed(KeyCode::ArrowRight) {
//             -10.0
//         } else {
//             0.0
//         };
//     }

//     ctrl.plus_x.use_rcs = docking_mode;
//     ctrl.plus_y.use_rcs = docking_mode;
//     ctrl.neg_x.use_rcs = docking_mode;
//     ctrl.neg_y.use_rcs = docking_mode;

//     ctrl
// }
