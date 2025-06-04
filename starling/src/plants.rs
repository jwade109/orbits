use crate::math::{rand, randint, rotate, Vec2, PI};

#[derive(Debug, Clone)]
pub struct Segment {
    length: f32,
    angle: f32,
    angular_velocity: f32,
    rest_angle: f32,
    children: Vec<Segment>,
    rigidity: f32,
    damping: f32,
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
                .map(|(angle, length)| {
                    let children = if length > 4.0 {
                        let n = randint(2, 5);
                        (0..n)
                            .map(|_| Segment {
                                length: rand(5.0, 9.5),
                                angle: rand(-PI, PI),
                                angular_velocity: rand(-0.05, 0.05),
                                rest_angle: rand(-0.9, 0.9),
                                children: Vec::new(),
                                rigidity: rand(3.0, 12.0),
                                damping: rand(3.0, 10.0),
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };

                    Segment {
                        length,
                        angle,
                        angular_velocity: rand(-0.05, 0.05),
                        rest_angle: rand(-0.2, 0.2),
                        children,
                        rigidity: rand(3.0, 12.0),
                        damping: rand(3.0, 10.0),
                    }
                })
                .collect(),
        }
    }

    pub fn step(&mut self, dt: f32, offset: f32) {
        for segment in &mut self.segments {
            let offset = offset + rand(-0.05, 0.05);

            let da = segment.rest_angle - segment.angle + offset;

            let accel = da * segment.rigidity - segment.angular_velocity * segment.damping;

            segment.angular_velocity += accel * dt / segment.length;

            segment.angle += segment.angular_velocity * dt;

            for c in &mut segment.children {
                let da = c.rest_angle - c.angle + offset;
                let accel = da * c.rigidity - c.angular_velocity * c.damping;
                c.angular_velocity += accel * dt / c.length;
                c.angle += c.angular_velocity * dt;
            }
        }
    }

    pub fn segments(&self) -> Vec<(Vec2, Vec2)> {
        let mut p = self.root;
        let mut angle = 0.0;
        let mut ret = Vec::new();
        for segment in &self.segments {
            let off = rotate(Vec2::Y * segment.length, angle + segment.angle);
            let q = p + off;
            ret.push((p, q));

            for s in &segment.children {
                let off = rotate(Vec2::Y * s.length, angle + s.angle);
                let q = p + off;
                ret.push((p, q));
            }

            angle += segment.angle;
            p = q;
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use lsystem::{LSystem, MapRules};

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
