use crate::math::{rand, rotate, Vec2};

#[derive(Debug, Copy, Clone)]
pub struct Segment {
    length: f32,
    angle: f32,
    angular_velocity: f32,
    rest_angle: f32,
}

#[derive(Debug)]
pub struct Plant {
    pub root: Vec2,
    pub segments: Vec<Segment>,
}

impl Plant {
    pub fn new(root: Vec2, segments: Vec<(f32, f32)>) -> Self {
        Self {
            root,
            segments: segments
                .into_iter()
                .map(|(angle, length)| Segment {
                    length,
                    angle,
                    angular_velocity: rand(-0.05, 0.05),
                    rest_angle: rand(-0.2, 0.2),
                })
                .collect(),
        }
    }

    pub fn step(&mut self, dt: f32, offset: f32) {
        for segment in &mut self.segments {
            let damping = 7.0;
            let rigidity: f32 = 9.0;

            let da = segment.rest_angle - segment.angle + offset;

            let accel = da * rigidity - segment.angular_velocity * damping;

            segment.angular_velocity += accel * dt / segment.length;

            segment.angle += segment.angular_velocity * dt;
        }
    }

    pub fn segments(&self) -> Vec<(Vec2, Vec2)> {
        let mut p = self.root;
        let mut angle = 0.0;
        let mut ret = Vec::new();
        for segment in &self.segments {
            angle += segment.angle;
            let off = rotate(Vec2::Y * segment.length, angle);
            let q = p + off;
            ret.push((p, q));
            p = q;
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use lsystem::{LRules, LSystem, MapRules};

    #[test]
    fn lsystem() {
        let mut rules = MapRules::new();
        rules.set_str('A', "AB");
        rules.set_str('B', "A");
        let axiom = "A".chars().collect();
        let mut system = LSystem::new(rules, axiom);

        let out = system.next().unwrap();
        let expected: Vec<char> = "AB".chars().collect();
        assert_eq!(expected, out);

        let out = system.next().unwrap();
        let expected: Vec<char> = "ABA".chars().collect();
        assert_eq!(expected, out);

        let out = system.next().unwrap();
        let expected: Vec<char> = "ABAAB".chars().collect();
        assert_eq!(expected, out);
    }
}
