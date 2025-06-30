use crate::math::Vec2;

pub struct Bezier {
    points: Vec<Vec2>,
}

fn eval(mut points: Vec<Vec2>, t: f32) -> Vec2 {
    while points.len() > 1 {
        for i in 0..(points.len() - 1) {
            let p = points[i];
            let q = points[i+1];
            let interp = p.lerp(q, t);
            points[i] = interp;
        }
        points.pop();
    }
    points[0]
}

impl Bezier {
    pub fn new(points: Vec<Vec2>) -> Self {
        Self {
            points,
        }
    }

    pub fn eval(&self, t: f32) -> Vec2 {
        eval(self.points.clone(), t)
    }
}
