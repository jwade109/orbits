use bary_core::prelude::*;
use bary_input::InputState;
use bary_sim::Camera;
use rdev::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LegState {
    Grappled,
    Travelling,
    Retracted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtensionState {
    Overextended,
    Nominal,
    UnderExtended,
}

pub const MIN_LEG_LENGTH: f32 = 0.6;
pub const MAX_LEG_LENGTH: f32 = 3.0;

pub struct Leg {
    pub min_length: f32,
    pub max_length: f32,
    pub desired_angle: f32,
    pub foot_position: Vec2,
    pub target_foot_position: Vec2,
    pub state: LegState,
}

impl Leg {
    fn desired_length(&self) -> f32 {
        (self.max_length + self.min_length) / 2.0
    }

    pub fn desired_offset(&self, spider_angle: f32) -> Vec2 {
        rotate(Vec2::X, self.desired_angle + spider_angle) * self.desired_length()
    }

    fn desired_foot_pos(&self, spider: Isometry2d) -> Vec2 {
        self.desired_offset(spider.rotation) + spider.translation
    }

    fn should_replant(&self, spider: Isometry2d) -> bool {
        let p = self.desired_foot_pos(spider);
        let a = self.foot_position;
        let dist_from_desired = p.distance(a);
        dist_from_desired > MIN_LEG_LENGTH
            || self.extension_state(spider.translation) != ExtensionState::Nominal
    }

    fn update(&mut self, dt: f32) {
        if self.state == LegState::Travelling {
            let max_speed = 65.0;
            let max_delta = max_speed * dt;
            let u = self.target_foot_position - self.foot_position;
            let delta = max_delta.min(u.length());
            let u = u.normalize_or_zero();
            let delta = u * delta;
            self.foot_position += delta;
            if delta.length() < 0.01 {
                self.state = LegState::Grappled;
            }
        }
    }

    pub fn replant(&mut self, spider: Isometry2d) {
        if self.state == LegState::Retracted {
            return;
        }
        let c = self.foot_position;
        let p = self.desired_foot_pos(spider);
        let u = p - c;
        let u = u.normalize_or_zero() * u.length().clamp(0.0, MIN_LEG_LENGTH);
        self.target_foot_position = p + u * 0.7; // vround(p / 2.0).as_vec2() * 2.0;
        self.state = LegState::Travelling;
    }

    fn extension_state(&self, spider: Vec2) -> ExtensionState {
        let l = spider.distance(self.foot_position);
        if l < self.min_length {
            ExtensionState::UnderExtended
        } else if l > self.max_length {
            ExtensionState::Overextended
        } else {
            ExtensionState::Nominal
        }
    }
}

pub struct Spider {
    pub pose: Isometry2d,
    pub vel: Isometry2d,
    pub legs: Vec<Leg>,
    pub is_drifting: bool,
}

fn make_legs(n: usize) -> Vec<Leg> {
    let angle_offset = bary_core::prelude::PI / n as f32;

    (0..n)
        .map(|i| {
            let a = angle_offset + i as f32 / n as f32 * 2.0 * bary_core::prelude::PI;
            Leg {
                desired_angle: a,
                min_length: MIN_LEG_LENGTH,
                max_length: MAX_LEG_LENGTH,
                foot_position: Vec2::ZERO,
                target_foot_position: rotate(Vec2::X, a) * 2.0,
                state: LegState::Travelling,
            }
        })
        .collect()
}

impl Spider {
    fn new(angle: f32, n_legs: usize) -> Self {
        Self {
            pose: Isometry2d::from_xya(0.0, 0.0, angle),
            vel: Isometry2d::ZERO,
            legs: make_legs(n_legs),
            is_drifting: false,
        }
    }

    pub fn toggle_drifting(&mut self) {
        let new_state = if self.is_drifting {
            LegState::Travelling
        } else {
            LegState::Retracted
        };

        for leg in &mut self.legs {
            leg.state = new_state;
        }

        self.is_drifting ^= true;

        if !self.is_drifting {
            for leg in &mut self.legs {
                leg.foot_position = self.pose.translation;
                leg.target_foot_position = leg.desired_foot_pos(self.pose);
                leg.state = LegState::Travelling;
            }
        }
    }

    fn worst_leg(&self) -> Option<usize> {
        self.legs
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                let d = l.desired_foot_pos(self.pose);
                let p = l.foot_position;
                let d = d.distance(p);
                (l.should_replant(self.pose)).then(|| (i, d))
            })
            .max_by(|x, y| x.1.total_cmp(&y.1))
            .map(|e| e.0)
    }
}

pub struct World {
    pub ticks: u64,
    pub time: f32,
    pub spiders: Vec<Spider>,
    pub camera: Camera,
}

fn update_spider(spider: &mut Spider, dt: f32) {
    let n_moving = spider
        .legs
        .iter()
        .filter(|l| l.state == LegState::Travelling)
        .count();

    if n_moving < 2 {
        if let Some(i) = spider.worst_leg() {
            spider.legs[i].replant(spider.pose);
        }
    }

    for leg in &mut spider.legs {
        leg.update(dt);
    }

    if spider.vel.translation.length() > 0.0 {
        let angle = spider.vel.translation.to_angle();
        let angle_vel = (angle - spider.pose.rotation) * 10.0;
        spider.vel.rotation = angle_vel;
    } else {
        spider.vel.rotation = 0.0;
    }

    spider.pose.translation += spider.vel.translation * dt;
    spider.pose.rotation += spider.vel.rotation * dt;
}

pub fn update_world(world: &mut World, dt: f32) {
    world.time += dt;
    world.ticks += 1;

    for spider in &mut world.spiders {
        update_spider(spider, dt);
    }

    let vel = Vec2::new(world.time.cos(), world.time.sin()) * 10.0;

    world.spiders[1].vel.translation = vel;

    world.camera.isometry.translation +=
        (world.spiders[0].pose.translation - world.camera.isometry.translation) * 1.0;
}

pub fn make_world() -> World {
    World {
        ticks: 0,
        time: 0.0,
        spiders: vec![Spider::new(0.4, 6), Spider::new(0.0, 3)],
        camera: Camera {
            isometry: Isometry2d::ZERO,
            zoom: 30.0,
        },
    }
}

pub fn process_input(world: &mut World, input: &InputState) {
    if input.is_key_pressed(Key::Minus) {
        world.camera.zoom /= 1.03;
        world.camera.zoom = world.camera.zoom.clamp(1.0, 70.0);
    }
    if input.is_key_pressed(Key::Equal) {
        world.camera.zoom *= 1.03;
        world.camera.zoom = world.camera.zoom.clamp(1.0, 70.0);
    }

    let n = input.is_key_pressed(Key::KeyW);
    let w = input.is_key_pressed(Key::KeyA);
    let s = input.is_key_pressed(Key::KeyS);
    let e = input.is_key_pressed(Key::KeyD);

    let x_pull = -(w as i8) + e as i8;
    let y_pull = -(s as i8) + n as i8;

    let speed = 16.0;

    let dir = Vec2::new(x_pull as f32, y_pull as f32).normalize_or_zero();

    let vel = dir * speed;

    let spider = &mut world.spiders[0];

    spider.vel.translation = vel;

    if input.just_pressed_debounced(Key::Space) {
        spider.toggle_drifting();
    }
}
