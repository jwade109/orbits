use bevy::color::Srgba;
use bevy::math::Vec2;
use starling::aabb::AABB;
use starling::math::{apply, linspace};

const GRAPH_PADDING: f32 = 2.0;

pub struct Signal<'a> {
    graph: &'a Graph,
    x: &'a [f32],
    y: &'a [f32],
    color: Srgba,
    line: LineType,
}

impl<'a> Signal<'a> {
    pub fn points(&self) -> impl Iterator<Item = Vec2> + use<'_> {
        self.x.iter().zip(self.y.iter()).map(|(x, y)| {
            self.graph
                .bounds(GRAPH_PADDING)
                .to_normalized(Vec2::new(*x, *y))
        })
    }

    pub fn color(&self) -> Srgba {
        self.color
    }

    pub fn line(&self) -> LineType {
        self.line
    }
}

pub struct Graph {
    bounds: AABB,
    x: Vec<f32>,
    signals: Vec<(Vec<f32>, Srgba, LineType)>,
    points: Vec<Vec2>,
}

#[derive(Debug, Clone, Copy)]
pub enum LineType {
    Line,
    Points,
    Both,
}

impl Graph {
    pub fn linspace(a: f32, b: f32, n: usize) -> Self {
        Graph {
            bounds: AABB::from_wh(0.0, 0.0),
            x: linspace(a, b, n),
            signals: Vec::new(),
            points: Vec::new(),
        }
    }

    pub fn points(&self) -> impl Iterator<Item = Vec2> + use<'_> {
        self.points
            .iter()
            .map(|p| self.bounds(GRAPH_PADDING).to_normalized(*p))
    }

    pub fn add_point(&mut self, x: f32, y: f32) {
        let p = Vec2::new(x, y);
        self.bounds.include(&p);
        self.points.push(p);
    }

    pub fn bounds(&self, padding: f32) -> AABB {
        self.bounds.padded(padding)
    }

    pub fn add_func(&mut self, func: impl Fn(f32) -> f32, color: Srgba, line: LineType) {
        let y = apply(&self.x, func);
        self.x.iter().zip(y.iter()).for_each(|(x, y)| {
            if y.is_nan() {
                return;
            }
            self.bounds.include(&Vec2::new(*x, *y));
        });
        self.signals.push((y, color, line));
    }

    pub fn origin(&self) -> Vec2 {
        self.bounds.padded(GRAPH_PADDING).to_normalized(Vec2::ZERO)
    }

    pub fn signals(&self) -> impl Iterator<Item = Signal> + use<'_> {
        self.signals.iter().map(|(y, color, line)| Signal {
            graph: self,
            x: &self.x,
            y,
            color: *color,
            line: *line,
        })
    }
}
