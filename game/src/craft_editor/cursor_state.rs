use starling::prelude::*;

#[derive(Debug, Default)]
pub enum CursorState {
    #[default]
    None,
    Part(PartDefinition),
    Logistics(Vec<IVec2>),
}

impl CursorState {
    pub fn current_part(&self) -> Option<PartDefinition> {
        match self {
            Self::Part(proto) => Some(proto.clone()),
            _ => None,
        }
    }

    pub fn toggle_logistics(&mut self) {
        *self = match self {
            Self::Logistics(_) => Self::None,
            _ => Self::logistics(),
        };
    }

    pub fn logistics() -> Self {
        Self::Logistics(Vec::new())
    }

    pub fn append_pipe_control_point(&mut self, p: IVec2) {
        if let Self::Logistics(points) = self {
            let p = if let Some(q) = points.last() {
                let dx = (q.x - p.x).abs();
                let dy = (q.y - p.y).abs();
                if dx > dy {
                    p.with_y(q.y)
                } else {
                    p.with_x(q.x)
                }
            } else {
                p
            };
            points.push(p);
        }
    }
}
