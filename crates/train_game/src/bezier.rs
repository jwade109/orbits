use glam::DVec2;

pub struct BezierCurve {
    pub a: DVec2,
    pub b: DVec2,
    pub c: DVec2,
}

impl BezierCurve {
    pub fn new(a: impl Into<DVec2>, b: impl Into<DVec2>, c: impl Into<DVec2>) -> Self {
        Self {
            a: a.into(),
            b: b.into(),
            c: c.into(),
        }
    }

    pub fn eval(&self, t: f64) -> DVec2 {
        let m = self.a.lerp(self.b, t);
        let n = self.b.lerp(self.c, t);
        m.lerp(n, t)
    }
}
