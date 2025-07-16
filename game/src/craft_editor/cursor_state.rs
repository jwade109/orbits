use starling::prelude::*;

#[derive(Debug, Default)]
pub enum CursorState {
    #[default]
    None,
    Part(PartPrototype),
    Pipes,
}

impl CursorState {
    pub fn current_part(&self) -> Option<PartPrototype> {
        match self {
            Self::Part(proto) => Some(proto.clone()),
            _ => None,
        }
    }

    pub fn toggle_logistics(&mut self) {
        *self = match self {
            Self::Pipes => Self::None,
            _ => Self::Pipes,
        };
    }
}
