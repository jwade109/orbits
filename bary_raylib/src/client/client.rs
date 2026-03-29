use super::Chat;
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct GridLocation {
    pub grid_id: Ent,
    pub coord: PartCoord,
}

impl GridLocation {
    pub fn new(grid_id: Ent, coord: PartCoord) -> Self {
        Self { grid_id, coord }
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct SelectionInfo {
    pub hovered: Option<GridLocation>,
    pub selected: Vec<GridLocation>,
}

impl SelectionInfo {
    pub fn selecting(grid_id: Ent) -> Self {
        Self {
            hovered: None,
            selected: vec![GridLocation {
                grid_id,
                coord: PartCoord((-1000, -1000).into()),
            }],
        }
    }

    pub fn first_selected_grid(&self) -> Option<Ent> {
        self.selected.first().map(|e| e.grid_id)
    }
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
    pub select_start: Option<PartCoord>,
    pub hovered: Option<PartCoord>,
}

#[derive(Debug)]
pub struct FreeFlying {
    pub follow_vehicle: Option<Ent>,
    pub lock_rotation: bool,
    pub selection_info: SelectionInfo,
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

    pub fn free(&self) -> Option<&FreeFlying> {
        match self {
            Self::Free(free) => Some(free),
            _ => None,
        }
    }

    pub fn free_mut(&mut self) -> Option<&mut FreeFlying> {
        match self {
            Self::Free(free) => Some(free),
            _ => None,
        }
    }

    pub fn editor(&self) -> Option<&EditorState> {
        match self {
            Self::Editor(e) => Some(e),
            _ => None,
        }
    }

    pub fn editor_mut(&mut self) -> Option<&mut EditorState> {
        match self {
            Self::Editor(e) => Some(e),
            _ => None,
        }
    }
}

/// Information that doesn't *in general* need to be synchronized across
/// clients in multiplayer.
pub struct ClientSpecificInfo {
    pub chat: Chat,
    pub mouse_screen_position: Option<Vec2>,
    pub screen_dims: Vec2,
    pub viewport: Viewport,
    pub is_holding_shift: bool,
}

impl ClientSpecificInfo {
    pub fn new() -> Self {
        Self {
            chat: Chat::default(),
            mouse_screen_position: None,
            screen_dims: Vec2::new(1500.0, 900.0),
            viewport: Viewport::Free(FreeFlying {
                follow_vehicle: None,
                lock_rotation: false,
                selection_info: SelectionInfo::default(),
            }),
            is_holding_shift: false,
        }
    }

    pub fn hovered_grid_loc(&self) -> Option<GridLocation> {
        match &self.viewport {
            Viewport::Editor(e) => {
                let coord = e.hovered?;
                Some(GridLocation {
                    grid_id: e.vehicle,
                    coord,
                })
            }
            Viewport::Free(f) => f.selection_info.hovered,
        }
    }

    pub fn focused_grid_id(&self) -> Option<Ent> {
        match &self.viewport {
            Viewport::Editor(e) => Some(e.vehicle),
            Viewport::Free(f) => f.selection_info.first_selected_grid(),
        }
    }
}
