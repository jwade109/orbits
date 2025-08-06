use bevy::color::palettes::css::*;
use bevy::prelude::*;
use lazy_static::lazy_static;
use starling::prelude::*;

pub struct Signal<'a> {
    graph: &'a Graph,
    x: &'a [f64],
    y: &'a [f64],
    color: Srgba,
}

impl<'a> Signal<'a> {
    pub fn points(&self) -> impl Iterator<Item = DVec2> + use<'_> {
        self.x.iter().zip(self.y.iter()).map(|(x, y)| {
            self.graph
                .bounds()
                .to_normalized(aabb_stopgap_cast(DVec2::new(*x, *y)))
                .as_dvec2()
        })
    }

    pub fn color(&self) -> Srgba {
        self.color
    }
}

pub struct Graph {
    bounds: Option<AABB>,
    x: Vec<f64>,
    signals: Vec<(Vec<f64>, Srgba)>,
    points: Vec<DVec2>,
}

impl Graph {
    pub fn linspace(a: f64, b: f64, n: usize) -> Self {
        Graph {
            bounds: None,
            x: linspace_f64(a, b, n),
            signals: Vec::new(),
            points: Vec::new(),
        }
    }

    pub fn blank() -> Self {
        Graph {
            bounds: None,
            x: Vec::new(),
            signals: Vec::new(),
            points: Vec::new(),
        }
    }

    pub fn points(&self) -> impl Iterator<Item = DVec2> + use<'_> {
        self.points.iter().map(|p| {
            self.bounds()
                .to_normalized(aabb_stopgap_cast(*p))
                .as_dvec2()
        })
    }

    fn update_bounds(&mut self, p: DVec2) {
        if let Some(aabb) = &mut self.bounds {
            aabb.include(&aabb_stopgap_cast(p));
        } else {
            self.bounds =
                Some(AABB::from_arbitrary(aabb_stopgap_cast(p), aabb_stopgap_cast(p)).padded(0.01));
        }
    }

    pub fn add_point(&mut self, x: f64, y: f64, update_bounds: bool) {
        let p = DVec2::new(x, y);
        if update_bounds {
            self.update_bounds(p);
        }
        self.points.push(p);
    }

    pub fn bounds(&self) -> AABB {
        self.bounds.unwrap_or(AABB::with_padding(0.1))
    }

    pub fn add_func(&mut self, func: impl Fn(f64) -> f64, color: Srgba) {
        let y = apply(&self.x, func);
        self.x.clone().iter().zip(y.iter()).for_each(|(x, y)| {
            if y.is_nan() {
                return;
            }
            self.update_bounds(DVec2::new(*x, *y));
        });
        self.signals.push((y, color));
    }

    pub fn origin(&self) -> DVec2 {
        self.bounds()
            .to_normalized(aabb_stopgap_cast(DVec2::ZERO))
            .as_dvec2()
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

#[allow(unused)]
pub fn get_orbit_info_graph(orbit: &SparseOrbit) -> Graph {
    let mut graph = Graph::linspace(-0.1, 1.1, 500);

    let t0 = orbit.epoch;
    let dur = Nanotime::secs(120);

    if let Some(tp) = orbit.t_next_p(orbit.epoch) {
        let y = orbit.periapsis_r() as f64;
        let period = orbit.period_or(Nanotime::zero());
        for i in -3..=12 {
            let x = (tp - orbit.epoch + period * i).to_secs_f64() / dur.to_secs_f64();
            graph.add_point(x, y, false)
        }
    };

    let t = |s: f64| t0 + dur * s;

    let pv = |s: f64| orbit.pv(t(s)).ok();
    let ta = |s: f64| orbit.ta_at_time(t(s)).unwrap_or(f64::NAN);

    let x1 = |s: f64| pv(s).map(|pv| pv.pos.x).unwrap_or(f64::NAN);
    let x2 = |s: f64| orbit.position_at(ta(s)).x;

    let y1 = |s: f64| pv(s).map(|pv| pv.pos.y).unwrap_or(f64::NAN);
    let y2 = |s: f64| orbit.position_at(ta(s)).y;

    let r = |s: f64| orbit.position_at(ta(s)).length() as f64;

    graph.add_func(x1, RED);
    // graph.add_func(x2, RED.with_green(0.4));
    graph.add_func(y1, TEAL);
    // graph.add_func(y2, TEAL.with_green(0.4));
    graph.add_func(r, ORANGE);

    graph
}

#[allow(unused)]
pub fn get_lut_error_graph(orbit: &SparseOrbit) -> Option<Graph> {
    let mut graph = Graph::linspace(-0.1 * PI_64, 2.1 * PI_64, 5000);

    let period = orbit.period()?;
    let tp = orbit.t_next_p(orbit.epoch)?;

    let t_at_x = |x: f64| {
        let s = x / (2.0 * PI_64);
        tp + period * s
    };

    let get_x = |pv: Option<PV>| pv.map(|pv| pv.pos.x).unwrap_or(f64::NAN);
    let get_y = |pv: Option<PV>| pv.map(|pv| pv.pos.y).unwrap_or(f64::NAN);

    let pv_slow_x = |x| get_x(orbit.pv_universal(t_at_x(x)).ok());
    let pv_slow_y = |x| get_y(orbit.pv_universal(t_at_x(x)).ok());

    let pv_lut_x = |x| get_x(orbit.pv_lut(t_at_x(x)));
    let pv_lut_y = |x| get_y(orbit.pv_lut(t_at_x(x)));

    let ra = orbit.apoapsis_r() as f64;

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
    let mut graph = Graph::linspace(-0.1 * PI_64, 2.1 * PI_64, 1000);

    graph.add_point(0.0, 0.0, true);
    graph.add_point(PI_64, 0.0, true);
    graph.add_point(2.0 * PI_64, 0.0, true);
    graph.add_point(0.0, PI_64, true);
    graph.add_point(0.0, 2.0 * PI_64, true);
    graph.add_point(2.0 * PI_64, 2.0 * PI_64, true);

    for ecc in linspace(0.0, 0.9, 10) {
        let f = |x| lookup_ta_from_ma(x as f64, ecc as f64).unwrap_or(f64::NAN) as f64;
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
