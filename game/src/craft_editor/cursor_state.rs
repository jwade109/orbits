use starling::prelude::*;

#[derive(Debug, Default)]
pub enum CursorState {
    #[default]
    None,
    Part(Part),
    Pipes,
}

impl CursorState {
    pub fn current_part(&self) -> Option<Part> {
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
