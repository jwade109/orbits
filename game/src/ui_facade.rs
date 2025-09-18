use crate::prelude::*;
use crate::text_button::ButtonId;
use starling::prelude::*;

pub struct UiFacade {
    current_vehicle_controller: Option<VehicleControlPolicy>,
    screen_bounds: Vec2,
    is_rcs: bool,
}

impl Default for UiFacade {
    fn default() -> Self {
        Self {
            current_vehicle_controller: None,
            screen_bounds: Vec2::ZERO,
            is_rcs: false,
        }
    }
}

fn current_vehicle_controller(state: &GameState) -> Option<UiFacade> {
    let id = state.piloting()?;
    let sv = state.universe.spacecraft.get(&id)?;

    Some(UiFacade {
        current_vehicle_controller: Some(sv.controller.mode().clone()),
        screen_bounds: state.input.screen_bounds.span,
        is_rcs: sv.is_rcs_mode(),
    })
}

impl UiFacade {
    pub fn new(state: &GameState) -> Self {
        current_vehicle_controller(state).unwrap_or(Self::default())
    }

    pub fn is_piloting(&self) -> bool {
        self.current_vehicle_controller.is_some()
    }

    pub fn get_screen_bounds(&self) -> Vec2 {
        self.screen_bounds
    }

    pub fn get_state(&self, id: ButtonId) -> bool {
        match id {
            ButtonId::Editor => false,
            ButtonId::Rcs => self.is_rcs,
            ButtonId::Idle => self
                .current_vehicle_controller
                .as_ref()
                .map(|p| p.is_idle())
                .unwrap_or(false),
            ButtonId::Prograde => self
                .current_vehicle_controller
                .as_ref()
                .map(|p| p.is_prograde())
                .unwrap_or(false),
            ButtonId::Retrograde => self
                .current_vehicle_controller
                .as_ref()
                .map(|p| p.is_retrograde())
                .unwrap_or(false),
            ButtonId::Attitude => self
                .current_vehicle_controller
                .as_ref()
                .map(|p| p.is_attitude_hold())
                .unwrap_or(false),
            ButtonId::Position => self
                .current_vehicle_controller
                .as_ref()
                .map(|p| p.is_position_hold())
                .unwrap_or(false),
            ButtonId::Launch => self
                .current_vehicle_controller
                .as_ref()
                .map(|p| p.is_launch_to_orbit())
                .unwrap_or(false),
        }
    }
}
