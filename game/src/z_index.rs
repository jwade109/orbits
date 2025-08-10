#[derive(Debug, Clone, Copy)]
pub enum ZOrdering {
    Orbit,
    Planet,
    Factory,
    Shipscope,
    ThrustParticles,
    Vehicle,
    EditorInteriorPart,
    EditorPipe,
    EditorPipeJoint,
    EditorTankFill,
    EditorItemBackground,
    EditorItem,
    EditorStructuralPart,
    EditorExteriorPart,
    EditorConnGroupHighlight,
    EditorConflictHighlight,
    EditorMouseoverPartHighlight,
    EditorConbot,
    EditorWeldingParticles,
    EditorCursor,
    Text,
    HudIcon,
    ScaleIndicator,
}

impl ZOrdering {
    pub fn as_u32(&self) -> u32 {
        *self as u32
    }

    pub fn as_f32(&self) -> f32 {
        self.as_u32() as f32 / 1000.0
    }
}
