#![allow(unused)]

use bary_core::prelude::*;
use bary_input::InputState;
use bary_ipc::new_message_queue;
use bary_raylib::*;
use bary_sim::Camera;
use raylib::prelude::*;
use rdev::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LegState {
    Grappled,
    Travelling,
    Retracted,
}

const LEG_RADIUS: f32 = 2.0;

struct Leg {
    length: f32,
    angle: f32,
    foot_position: Vec2,
    target_foot_position: Vec2,
    state: LegState,
    spring_constant: f32,
}

impl Leg {
    fn desired_offset(&self, spider_angle: f32) -> Vec2 {
        rotate(Vec2::X, self.angle + spider_angle) * self.length
    }

    fn desired_foot_pos(&self, spider: Isometry2d) -> Vec2 {
        self.desired_offset(spider.rotation) + spider.translation
    }

    fn spring_force(&self, spider_pos: Vec2) -> Vec2 {
        if self.state == LegState::Grappled {
            let l = spider_pos.distance(self.foot_position) - self.length;
            let u = (self.foot_position - spider_pos).normalize_or_zero();
            u * l
        } else {
            Vec2::ZERO
        }
    }

    fn should_replant(&self, spider: Isometry2d) -> bool {
        let d = self.desired_foot_pos(spider);
        let l = self.foot_position.distance(d);
        l > LEG_RADIUS
    }

    fn update(&mut self) {
        if self.state == LegState::Travelling {
            let delta = (self.target_foot_position - self.foot_position);
            self.foot_position += delta;
            self.state = LegState::Grappled;
        }
    }

    fn replant(&mut self, spider: Isometry2d) {
        if self.state == LegState::Retracted {
            return;
        }
        let p = self.desired_foot_pos(spider);
        self.target_foot_position = p;
        self.state = LegState::Travelling;
    }
}

struct Spider {
    pose: Isometry2d,
    vel: Isometry2d,
    legs: Vec<Leg>,
    mass: f32,
    is_drifting: bool,
}

impl Spider {
    fn new(angle: f32) -> Self {
        let legs = (0..6)
            .map(|i| {
                let a = i as f32 / 6.0 * 2.0 * bary_core::prelude::PI;
                Leg {
                    angle: a,
                    length: 2.0,
                    foot_position: Vec2::ZERO,
                    target_foot_position: rotate(Vec2::X, a) * 2.0,
                    state: LegState::Travelling,
                    spring_constant: 50.0,
                }
            })
            .collect();

        Self {
            pose: Isometry2d::from_xya(0.0, 0.0, angle),
            vel: Isometry2d::ZERO,
            legs,
            mass: 1.0,
            is_drifting: false,
        }
    }

    fn toggle_drifting(&mut self) {
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

struct World {
    spiders: Vec<Spider>,
    camera: Camera,
}

fn draw_spider(d: &mut RaylibDrawHandle, spider: &Spider) {
    for leg in &spider.legs {
        if leg.state == LegState::Retracted {
            continue;
        }
        let offset = leg.desired_offset(spider.pose.rotation);
        let s = spider.pose.translation;
        let e = leg.foot_position;
        let color = if leg.should_replant(spider.pose) {
            Color::RED
        } else {
            Color::GREEN
        };
        draw_line_width(d, s, e, 0.2, Color::GRAY);
        fill_circle(d, e, 0.2, Color::GRAY);
        draw_circle(d, e, LEG_RADIUS, Color::RED);

        if leg.state == LegState::Travelling {
            let p = leg.target_foot_position;
            // fill_circle(d, p, 0.15, Color::YELLOW);
        }
    }

    let body_dims = Vec2::new(1.6, 1.1);
    let mut body_iso = spider.pose;
    body_iso = body_iso.offset(-body_dims / 2.0);
    fill_rectangle(d, body_iso, body_dims, Color::TEAL);
}

fn draw_world(handle: &mut RaylibDrawHandle, world: &World) {
    let screen_dims = Vec2::new(
        handle.get_screen_width() as f32,
        handle.get_screen_height() as f32,
    );

    let cam = to_raylib_camera(&world.camera, screen_dims);
    let mut draw_handle = handle.begin_mode2D(cam);
    let d = &mut draw_handle;

    for x in (-100..100).step_by(10) {
        let s = Vec2::new(x as f32, -1000.0);
        let e = Vec2::new(x as f32, 1000.0);
        draw_line(d, s, e, Color::GRAY);
        let s = Vec2::new(-1000.0, x as f32);
        let e = Vec2::new(1000.0, x as f32);
        draw_line(d, s, e, Color::GRAY);
    }

    for spider in &world.spiders {
        draw_spider(d, spider);
    }
}

fn update_spider(spider: &mut Spider, dt: f32) {
    let mut any_legs_moving = spider.legs.iter().any(|l| l.state == LegState::Travelling);
    let mut dv = Vec2::ZERO;

    for leg in &mut spider.legs {
        leg.update();

        let force = leg.spring_force(spider.pose.translation);
        let a = force / spider.mass;
        dv += a;
    }

    if !any_legs_moving {
        if let Some(i) = spider.worst_leg() {
            spider.legs[i].replant(spider.pose);
        }
    }

    for leg in &spider.legs {
        if leg.state == LegState::Grappled {
            spider.vel.translation *= 0.99;
        }
    }

    spider.vel.translation += dv;
    spider.pose.translation += spider.vel.translation * dt;
    spider.pose.rotation += spider.vel.rotation * dt;
}

fn update_world(world: &mut World, dt: f32) {
    for spider in &mut world.spiders {
        update_spider(spider, dt);
    }

    world.camera.isometry.translation +=
        (world.spiders[0].pose.translation - world.camera.isometry.translation) * 1.0;
}

fn make_world() -> World {
    World {
        spiders: vec![Spider::new(0.4), Spider::new(0.0)],
        camera: Camera {
            isometry: Isometry2d::ZERO,
            zoom: 70.0,
        },
    }
}

fn process_input(world: &mut World, input: &InputState) {
    let n = input.is_key_pressed(Key::KeyW);
    let w = input.is_key_pressed(Key::KeyA);
    let s = input.is_key_pressed(Key::KeyS);
    let e = input.is_key_pressed(Key::KeyD);

    let l = input.is_key_pressed(Key::KeyQ);
    let r = input.is_key_pressed(Key::KeyE);

    let x_pull = -(w as i8) + e as i8;
    let y_pull = -(s as i8) + n as i8;

    let speed = 4.0;

    let dir = Vec2::new(x_pull as f32, y_pull as f32).normalize_or_zero();

    let force = dir * speed;

    let mut spider = &mut world.spiders[0];

    let rot_dir = l as i8 - r as i8;

    let acc = force / spider.mass;
    spider.vel.translation += acc;

    spider.vel.rotation = rot_dir as f32 * 1.2;

    if input.just_pressed_debounced(Key::Space) {
        println!("Toggle drifting!");
        spider.toggle_drifting();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut handle, thread) = raylib::init()
        .size(1080, 700)
        .title("Spider Demo")
        .log_level(TraceLogLevel::LOG_WARNING)
        .msaa_4x()
        .resizable()
        .build();

    handle.set_target_fps(120);
    handle.maximize_window();

    let mut input = InputState::default();

    let input_queue = new_message_queue();
    let thread_copy = input_queue.clone();

    let _input_thread = std::thread::spawn(|| {
        if let Err(error) = rdev::listen(move |e| thread_copy.push(e)) {
            println!("Error: {:?}", error)
        }
    });

    let mut world = make_world();

    while !handle.window_should_close() {
        let dt = handle.get_frame_time();

        while let Some(event) = input_queue.pop() {
            input.process_rdev_event(&event, handle.is_window_focused());
        }

        process_input(&mut world, &input);
        update_world(&mut world, dt);

        handle.draw(&thread, |mut d| {
            d.clear_background(Color::BLACK);
            draw_world(&mut d, &world);
        });

        if input.is_key_pressed(Key::Escape) {
            break;
        }

        if input.is_key_pressed(Key::Minus) {
            world.camera.zoom /= 1.03;
            world.camera.zoom = world.camera.zoom.clamp(1.0, 100.0);
        }

        if input.is_key_pressed(Key::Equal) {
            world.camera.zoom *= 1.03;
            world.camera.zoom = world.camera.zoom.clamp(1.0, 100.0);
        }

        input.on_frame_boundary();
    }

    Ok(())
}
