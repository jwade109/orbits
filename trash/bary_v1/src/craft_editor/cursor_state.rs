use bary_core::prelude::*;

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

#[derive(Debug, Default, Clone)]
pub struct SelectedState {
    start: Option<DVec2>,
    end: Option<DVec2>,
}

impl SelectedState {
    pub fn update_start(&mut self, pos: impl Into<Option<DVec2>>) {
        self.start = pos.into();
    }

    pub fn update_end(&mut self, pos: impl Into<Option<DVec2>>) {
        self.end = pos.into();
    }

    pub fn cells(&self) -> impl Iterator<Item = PartCoord> + use<> {
        self.aabb()
            .map(|aabb| PartCoord::in_aabb(aabb))
            .into_iter()
            .flatten()
    }

    pub fn aabb(&self) -> Option<AABB> {
        let a = self.start?.as_vec2();
        let b = self.end?.as_vec2();
        Some(AABB::from_arbitrary(a, b))
    }
}

#[derive(Debug, Default)]
pub enum CursorState {
    #[default]
    None,
    Part(String),
    Blueprint(Blueprint),
    Pipe(CursorPipeData),
    Select(SelectedState),
}

impl CursorState {
    pub fn current_part(&self) -> Option<&String> {
        match self {
            Self::Part(proto) => Some(proto),
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

    pub fn selected(&self) -> Option<&SelectedState> {
        match self {
            Self::Select(sel) => Some(sel),
            _ => None,
        }
    }

    pub fn sel_mut(&mut self) -> Option<&mut SelectedState> {
        match self {
            Self::Select(sel) => Some(sel),
            _ => None,
        }
    }
}
