use bevy::color::Srgba;
use bevy::math::Vec2;
use starling::aabb::AABB;

pub struct Signal<'a> {
    graph: &'a Graph,
    points: &'a Vec<Vec2>,
    color: Srgba,
}

impl<'a> Signal<'a> {
    pub fn points(&self) -> impl Iterator<Item = Vec2> + use<'_> {
        self.points
            .iter()
            .map(|p| self.graph.bounds(0.1).to_normalized(*p))
    }

    pub fn color(&self) -> Srgba {
        self.color
    }
}

pub struct Graph {
    bounds: AABB,
    signals: Vec<(Vec<Vec2>, Srgba)>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            bounds: AABB::from_wh(0.0, 0.0),
            signals: Vec::new(),
        }
    }

    pub fn bounds(&self, padding: f32) -> AABB {
        self.bounds.padded(padding)
    }

    pub fn add_signal(&mut self, signal: Vec<Vec2>, color: Srgba) {
        for p in &signal {
            self.bounds.include(p);
        }
        self.signals.push((signal, color))
    }

    pub fn add_xy(&mut self, x: &[f32], y: &[f32], color: Srgba) {
        let points = x
            .iter()
            .zip(y.iter())
            .map(|(x, y)| Vec2::new(*x, *y))
            .collect();
        self.add_signal(points, color);
    }

    pub fn signals(&self) -> impl Iterator<Item = Signal> + use<'_> {
        self.signals.iter().map(|(points, color)| Signal {
            graph: self,
            points,
            color: *color,
        })
    }
}
