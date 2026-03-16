use super::Chat;
use crate::sim::PartOccupancy;
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct SelectionInfo {
    pub camera_hovered: Option<Ent>,
    pub mouse_hovered: Option<Ent>,
    pub selected_grid: Option<Ent>,
    pub mouseover_part_info: Option<(PartCoord, PartOccupancy)>,
}

pub struct ClientSpecificInfo {
    pub chat: Chat,
    pub mouse_screen_position: Option<Vec2>,
    pub screen_dims: Vec2,
    pub selection_info: SelectionInfo,
}

impl ClientSpecificInfo {
    pub fn new() -> Self {
        Self {
            chat: Chat::default(),
            mouse_screen_position: None,
            screen_dims: Vec2::new(1500.0, 900.0),
            selection_info: SelectionInfo::default(),
        }
    }
}
