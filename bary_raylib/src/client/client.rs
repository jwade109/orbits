use super::Chat;
use crate::{camera::Camera, sim::PartOccupancy};
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct SelectionInfo {
    pub camera_hovered: Option<Ent>,
    pub mouse_hovered: Option<Ent>,
    pub selected_grid: Option<Ent>,
    pub mouseover_part_info: Option<(PartCoord, PartOccupancy)>,
}

#[derive(Debug)]
pub struct EditorState {
    pub vehicle: Ent,
    pub camera_offset: Vec2,
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
    pub fn look_at(&mut self, id: Ent) {
        if let Self::Free(fly) = self {
            fly.follow_vehicle = Some(id);
        }
    }
}

pub struct ClientSpecificInfo {
    pub chat: Chat,
    pub mouse_screen_position: Option<Vec2>,
    pub screen_dims: Vec2,
    pub selection_info: SelectionInfo,
    pub viewport: Viewport,
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
        }
    }
}
