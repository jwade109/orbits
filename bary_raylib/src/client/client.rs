use super::Chat;
use crate::sim::PartOccupancy;
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct SelectionInfo {
    pub mouse_hovered: Option<Ent>,
    pub selected_grid: Option<Ent>,
    pub mouseover_part_info: Option<(PartCoord, PartOccupancy)>,
}

#[derive(Debug)]
pub struct EditorState {
    pub vehicle: Ent,
    pub target_offset: Vec2,
    pub actual_offset: Vec2,
    pub camera_rotation: Rotation,
    pub prototype_id: Option<Ent>,
    pub part_rotation: Rotation,
    pub layer: Option<PartLayer>,
}

#[derive(Debug)]
pub struct FreeFlying {
    pub follow_vehicle: Option<Ent>,
    pub lock_rotation: bool,
}

#[derive(Debug)]
pub enum Viewport {
    Editor(EditorState),
    Free(FreeFlying),
}

impl Viewport {
    pub fn look_at(&mut self, id: Ent) -> bool {
        if let Self::Free(fly) = self {
            let ret = fly.follow_vehicle != Some(id);
            fly.follow_vehicle = Some(id);
            ret
        } else {
            false
        }
    }

    pub fn is_real_view(&self) -> bool {
        match self {
            Self::Free(_) => true,
            _ => false,
        }
    }
}

/// Information that doesn't *in general* need to be synchronized across
/// clients in multiplayer.
pub struct ClientSpecificInfo {
    pub chat: Chat,
    pub mouse_screen_position: Option<Vec2>,
    pub screen_dims: Vec2,
    pub selection_info: SelectionInfo,
    pub viewport: Viewport,
    pub is_holding_shift: bool,
}

impl ClientSpecificInfo {
    pub fn new() -> Self {
        Self {
            chat: Chat::default(),
            mouse_screen_position: None,
            screen_dims: Vec2::new(1500.0, 900.0),
            selection_info: SelectionInfo::default(),
            viewport: Viewport::Free(FreeFlying {
                follow_vehicle: None,
                lock_rotation: false,
            }),
            is_holding_shift: false,
        }
    }
}
