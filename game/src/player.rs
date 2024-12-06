use core::f32;

use bevy::color::palettes::basic::*;
use bevy::prelude::*;

pub struct SpaceshipPlugin {}

impl Plugin for SpaceshipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_spaceship);
        app.add_systems(FixedUpdate, update_spaceship);
        app.add_systems(Update, (keyboard_input, render_spaceship));
    }
}

#[derive(Default, Copy, Clone)]
enum TurnState {
    #[default]
    None,
    Left,
    Right,
}

#[derive(Component, Default, Clone, Copy)]
struct Spaceship {
    position: Vec2,
    velocity: Vec2,
    rotation: f32,
    angular_velocity: f32,

    thrusting: bool,
    turning: TurnState,
}

impl Spaceship {
    fn at_position(pos: Vec2) -> Self {
        Spaceship {
            position: pos,
            velocity: Vec2::ZERO,
            rotation: 0.0,
            angular_velocity: 0.0,
            thrusting: false,
            turning: TurnState::None,
        }
    }

    fn pointing(&self) -> Vec2 {
        Vec2::from_angle(self.rotation)
    }

    fn step(&mut self, dt: f32) {
        let vel = self.velocity;
        self.position += vel * dt;
        self.velocity *= f32::exp(-dt / 4.0);
        self.rotation += self.angular_velocity * dt;
        self.angular_velocity *= f32::exp(-dt / 8.0);

        let angular_accel = match self.turning {
            TurnState::Left => 6.0,
            TurnState::Right => -6.0,
            TurnState::None => 0.0,
        };

        self.angular_velocity += angular_accel * dt;

        let linear_accel: Vec2 = match self.thrusting {
            true => 400.0 * self.pointing(),
            false => Vec2::ZERO,
        };

        self.velocity += linear_accel * dt;
    }

    fn predict(self, dt: f32, n: u32, m: u32) -> [Vec<Vec2>; 4] {
        let mut coasting = self;
        let mut no_turning = self;
        let mut left_turning: Spaceship = self;
        let mut right_turning = self;

        coasting.thrusting = false;

        no_turning.turning = TurnState::None;
        left_turning.turning = TurnState::Left;
        right_turning.turning = TurnState::Right;

        no_turning.thrusting = true;
        left_turning.thrusting = true;
        right_turning.thrusting = true;

        let sim = |turning: TurnState, thrusting: bool, iters: u32| -> Vec<Vec2> {
            let mut spaceship = self;
            spaceship.turning = turning;
            spaceship.thrusting = thrusting;
            (0..iters)
                .map(|_| {
                    let p = spaceship.position;
                    spaceship.step(dt);
                    p
                })
                .collect()
        };

        [
            sim(TurnState::None, false, n),
            sim(TurnState::None, true, n),
            sim(TurnState::Left, true, m),
            sim(TurnState::Right, true, m),
        ]
    }
}

fn spawn_spaceship(mut commands: Commands) {
    commands.spawn(Spaceship::at_position((200.0, 200.0).into()));
}

fn update_spaceship(time: Res<Time>, mut query: Query<&mut Spaceship>) {
    let dt: f32 = time.delta().as_secs_f32();
    for mut sp in query.iter_mut() {
        let vel = sp.velocity;
        sp.position += vel * dt;
        sp.velocity *= f32::exp(-dt / 4.0);
        sp.rotation += sp.angular_velocity * dt;
        sp.angular_velocity *= f32::exp(-dt / 8.0);

        let angular_accel = match sp.turning {
            TurnState::Left => 6.0,
            TurnState::Right => -6.0,
            TurnState::None => 0.0,
        };

        sp.angular_velocity += angular_accel * dt;

        let linear_accel = match sp.thrusting {
            true => 400.0 * sp.pointing(),
            false => Vec2::ZERO,
        };

        sp.velocity += linear_accel * dt;
    }
}

fn render_spaceship(time: Res<Time>, mut gizmos: Gizmos, query: Query<&Spaceship>) {
    for sp in query.iter() {
        let pointing = sp.pointing() * 20.0;
        let left = Vec2::from_angle(std::f32::consts::PI * 0.8).rotate(pointing);
        let right = Vec2::from_angle(-std::f32::consts::PI * 0.8).rotate(pointing);

        gizmos.linestrip_2d(
            [
                sp.position + pointing,
                sp.position + left,
                sp.position + right,
                sp.position + pointing,
            ],
            WHITE,
        );

        if sp.thrusting {
            let mag = 1.4 + 0.1 * f32::sin(time.elapsed_secs_wrapped() * 100.0);
            gizmos.linestrip_2d(
                [
                    sp.position + left,
                    sp.position - pointing * mag,
                    sp.position + right,
                ],
                WHITE,
            );
        }

        let iso = Isometry2d::new(
            sp.position,
            (sp.rotation - std::f32::consts::PI / 2.0).into(),
        );
        let arc_angle = std::f32::consts::PI / 3.0;
        let radius = 40.0;

        match sp.turning {
            TurnState::Left => {
                gizmos.arc_2d(iso, arc_angle, radius, WHITE);
            }
            TurnState::Right => {
                gizmos.arc_2d(iso, -arc_angle, radius, WHITE);
            }
            _ => (),
        }

        let predictions: [Vec<Vec2>; 4] = sp.predict(0.05, 60, 20);

        for (p, c) in predictions.into_iter().zip([GRAY, BLUE, RED, GREEN]) {
            gizmos.linestrip_2d(p, c);
        }
    }
}

fn keyboard_input(keys: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Spaceship>) {
    let mut thrusting = false;
    let mut turning = TurnState::None;

    for key in keys.get_pressed() {
        match key {
            KeyCode::ArrowDown | KeyCode::KeyS => {}
            KeyCode::ArrowUp | KeyCode::KeyW => {
                thrusting = true;
            }
            KeyCode::ArrowLeft | KeyCode::KeyA => {
                turning = TurnState::Left;
            }
            KeyCode::ArrowRight | KeyCode::KeyD => {
                turning = TurnState::Right;
            }
            _ => (),
        }
    }

    for mut sp in query.iter_mut() {
        sp.turning = turning;
        sp.thrusting = thrusting;
    }
}
