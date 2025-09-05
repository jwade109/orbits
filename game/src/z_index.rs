#[derive(Debug, Clone, Copy)]
pub enum ZOrdering {
    Starfield,
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
    HudAngularMomentum,
    ScaleIndicator,
    Transforms,
    Ui,
    Ui2,
    Ui3,
    Window(u32, u32),
}

impl ZOrdering {
    pub fn as_u32(&self) -> i32 {
        match self {
            ZOrdering::Starfield => -1,
            ZOrdering::Orbit => 0,
            ZOrdering::Planet => 1,
            ZOrdering::Factory => 2,
            ZOrdering::Shipscope => 3,
            ZOrdering::ThrustParticles => 4,
            ZOrdering::Vehicle => 5,
            ZOrdering::EditorInteriorPart => 6,
            ZOrdering::EditorPipe => 7,
            ZOrdering::EditorPipeJoint => 8,
            ZOrdering::EditorTankFill => 9,
            ZOrdering::EditorItemBackground => 10,
            ZOrdering::EditorItem => 11,
            ZOrdering::EditorStructuralPart => 12,
            ZOrdering::EditorExteriorPart => 13,
            ZOrdering::EditorConnGroupHighlight => 14,
            ZOrdering::EditorConflictHighlight => 15,
            ZOrdering::EditorMouseoverPartHighlight => 16,
            ZOrdering::EditorConbot => 17,
            ZOrdering::EditorWeldingParticles => 18,
            ZOrdering::EditorCursor => 19,
            ZOrdering::Text => 20,
            ZOrdering::HudIcon => 21,
            ZOrdering::HudAngularMomentum => 22,
            ZOrdering::ScaleIndicator => 23,
            ZOrdering::Transforms => 24,
            ZOrdering::Ui => 25,
            ZOrdering::Ui2 => 26,
            ZOrdering::Ui3 => 27,
            ZOrdering::Window(n, l) => (28 + n * 100 + l) as i32,
        }
    }

    pub fn as_f32(&self) -> f32 {
        self.as_u32() as f32 / 100.0
    }
}
