use bary_core::prelude::*;

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
