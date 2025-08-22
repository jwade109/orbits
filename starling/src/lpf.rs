#[derive(Debug, Clone)]
pub struct Lpf {
    pub target: f32,
    pub actual: f32,
    pub alpha: f32,
}

impl Lpf {
    pub fn new(target: f32, actual: f32, alpha: f32) -> Self {
        Self {
            target,
            actual,
            alpha,
        }
    }

    pub fn step(&mut self) {
        self.actual += (self.target - self.actual) * self.alpha;
    }
}
