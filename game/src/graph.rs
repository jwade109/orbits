use bevy::color::palettes::css::*;
use bevy::prelude::*;
use lazy_static::lazy_static;
use starling::prelude::*;

pub struct Signal<'a> {
    graph: &'a Graph,
    x: &'a [f32],
    y: &'a [f32],
    color: Srgba,
}

impl<'a> Signal<'a> {
    pub fn points(&self) -> impl Iterator<Item = Vec2> + use<'_> {
        self.x
            .iter()
            .zip(self.y.iter())
            .map(|(x, y)| self.graph.bounds().to_normalized(Vec2::new(*x, *y)))
    }

    pub fn color(&self) -> Srgba {
        self.color
    }
}

pub struct Graph {
    bounds: Option<AABB>,
    x: Vec<f32>,
    signals: Vec<(Vec<f32>, Srgba)>,
    points: Vec<Vec2>,
}

impl Graph {
    pub fn linspace(a: f32, b: f32, n: usize) -> Self {
        Graph {
            bounds: None,
            x: linspace(a, b, n),
            signals: Vec::new(),
            points: Vec::new(),
        }
    }

    pub fn points(&self) -> impl Iterator<Item = Vec2> + use<'_> {
        self.points.iter().map(|p| self.bounds().to_normalized(*p))
    }

    fn update_bounds(&mut self, p: Vec2) {
        if let Some(aabb) = &mut self.bounds {
            aabb.include(&p);
        } else {
            self.bounds = Some(AABB::from_arbitrary(p, p).padded(0.01));
        }
    }

    pub fn add_point(&mut self, x: f32, y: f32, update_bounds: bool) {
        let p = Vec2::new(x, y);
        if update_bounds {
            self.update_bounds(p);
        }
        self.points.push(p);
    }

    pub fn bounds(&self) -> AABB {
        self.bounds.unwrap_or(AABB::with_padding(0.1))
    }

    pub fn add_func(&mut self, func: impl Fn(f32) -> f32, color: Srgba) {
        let y = apply(&self.x, func);
        self.x.clone().iter().zip(y.iter()).for_each(|(x, y)| {
            if y.is_nan() {
                return;
            }
            self.update_bounds(Vec2::new(*x, *y));
        });
        self.signals.push((y, color));
    }

    pub fn origin(&self) -> Vec2 {
        self.bounds().to_normalized(Vec2::ZERO)
    }

    pub fn signals(&self) -> impl Iterator<Item = Signal> + use<'_> {
        self.signals.iter().map(|(y, color)| Signal {
            graph: self,
            x: &self.x,
            y,
            color: *color,
        })
    }
}

pub fn get_orbit_info_graph(orbit: &SparseOrbit) -> Graph {
    let mut graph = Graph::linspace(-0.1, 1.1, 800);

    let t0 = orbit.epoch;
    let period = Nanotime::secs(30);

    let t = |s: f32| t0 + period * s;

    let pv = |s: f32| orbit.pv(t(s)).ok();
    let ta = |s: f32| orbit.ta_at_time(t(s)).unwrap_or(f32::NAN);

    let x1 = |s: f32| pv(s).map(|pv| pv.pos.x).unwrap_or(f32::NAN);
    let x2 = |s: f32| orbit.position_at(ta(s)).x;

    let y1 = |s: f32| pv(s).map(|pv| pv.pos.y).unwrap_or(f32::NAN);
    let y2 = |s: f32| orbit.position_at(ta(s)).y;

    graph.add_func(x1, RED);
    graph.add_func(x2, RED.with_green(0.2));
    graph.add_func(y1, TEAL);
    graph.add_func(y2, TEAL.with_green(0.2));

    graph
}

#[allow(unused)]
pub fn get_lut_error_graph(orbit: &SparseOrbit) -> Option<Graph> {
    let mut graph = Graph::linspace(-0.1 * PI, 2.1 * PI, 5000);

    let period = orbit.period()?;
    let tp = orbit.t_next_p(orbit.epoch)?;

    let t_at_x = |x: f32| {
        let s = x / (2.0 * PI);
        tp + period * s
    };

    let get_x = |pv: Option<PV>| pv.map(|pv| pv.pos.x).unwrap_or(f32::NAN);
    let get_y = |pv: Option<PV>| pv.map(|pv| pv.pos.y).unwrap_or(f32::NAN);

    let pv_slow_x = |x| get_x(orbit.pv_universal(t_at_x(x)).ok());
    let pv_slow_y = |x| get_y(orbit.pv_universal(t_at_x(x)).ok());

    let pv_lut_x = |x| get_x(orbit.pv_lut(t_at_x(x)));
    let pv_lut_y = |x| get_y(orbit.pv_lut(t_at_x(x)));

    let ra = orbit.apoapsis_r();

    let error_x = |x| (pv_slow_x(x) - pv_lut_x(x)) / ra;
    let error_y = |x| (pv_slow_y(x) - pv_lut_y(x)) / ra;

    graph.add_func(error_x, TEAL);
    graph.add_func(error_y, LIME);

    graph.add_point(0.0, 0.001, false);
    graph.add_point(0.0, 0.01, false);
    graph.add_point(0.0, 0.1, false);
    graph.add_point(0.0, 1.0, false);

    Some(graph)
}

fn generate_lut_graph() -> Graph {
    let mut graph = Graph::linspace(-0.1 * PI, 2.1 * PI, 1000);

    graph.add_point(0.0, 0.0, true);
    graph.add_point(PI, 0.0, true);
    graph.add_point(2.0 * PI, 0.0, true);
    graph.add_point(0.0, PI, true);
    graph.add_point(0.0, 2.0 * PI, true);
    graph.add_point(2.0 * PI, 2.0 * PI, true);

    for ecc in linspace(0.0, 0.9, 10) {
        let f = |x| lookup_ta_from_ma(x, ecc).unwrap_or(f32::NAN);
        graph.add_func(f, GREEN.mix(&RED, ecc));
    }

    graph
}

lazy_static! {
    static ref LUT_GRAPH: Graph = generate_lut_graph();
}

pub fn get_lut_graph() -> &'static Graph {
    &LUT_GRAPH
}
