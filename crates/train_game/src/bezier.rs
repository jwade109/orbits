use glam::DVec2;

pub struct BezierCurve {
    pub points: Vec<DVec2>,
}

fn eval_once(points: Vec<DVec2>, t: f64) -> Vec<DVec2> {
    points.windows(2).map(|w| w[0].lerp(w[1], t)).collect()
}

impl BezierCurve {
    pub fn new(points: Vec<DVec2>) -> Self {
        Self { points }
    }

    pub fn eval(&self, t: f64) -> DVec2 {
        let mut p = self.points.clone();
        while p.len() > 1 {
            p = eval_once(p, t);
        }
        p[0]
    }
}
