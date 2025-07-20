use starling::prelude::*;

#[derive(Debug, Default)]
pub enum CursorState {
    #[default]
    None,
    Part(PartPrototype),
}

impl CursorState {
    pub fn current_part(&self) -> Option<PartPrototype> {
        match self {
            Self::Part(proto) => Some(proto.clone()),
            _ => None,
        }
    }
}
