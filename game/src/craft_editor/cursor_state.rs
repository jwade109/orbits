use crate::starling::prelude::*;

#[derive(Debug, Default, Clone)]
pub struct CursorPipeData {
    pub start_position: Option<PartCoord>,
    pub end_position: Option<PartCoord>,
    pub x_first: bool,
}

impl CursorPipeData {
    pub fn pipe_geometry(&self) -> Option<PipeGeometry> {
        if self.start_position == self.end_position {
            return None;
        }

        Some(PipeGeometry {
            start: self.start_position?,
            end: self.end_position?,
            x_first: self.x_first,
        })
    }
}

#[derive(Debug, Default)]
pub enum CursorState {
    #[default]
    None,
    Part(PartPrototype),
    Blueprint(Blueprint),
    Pipe(CursorPipeData),
}

impl CursorState {
    pub fn current_part(&self) -> Option<PartPrototype> {
        match self {
            Self::Part(proto) => Some(proto.clone()),
            _ => None,
        }
    }

    pub fn blueprint(&self) -> Option<&Blueprint> {
        match self {
            Self::Blueprint(bp) => Some(bp),
            _ => None,
        }
    }

    pub fn blueprint_mut(&mut self) -> Option<&mut Blueprint> {
        match self {
            Self::Blueprint(bp) => Some(bp),
            _ => None,
        }
    }

    pub fn pipe(&self) -> Option<&CursorPipeData> {
        match self {
            Self::Pipe(data) => Some(data),
            _ => None,
        }
    }

    pub fn pipe_mut(&mut self) -> Option<&mut CursorPipeData> {
        match self {
            Self::Pipe(data) => Some(data),
            _ => None,
        }
    }
}
