use bevy::color::palettes::basic::*;
use bevy::math::NormedVectorSpace;
use bevy::prelude::*;

use starling::core::rand;

pub struct BallsPlugin;

impl Plugin for BallsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, draw_balls);
        app.add_systems(FixedUpdate, update_balls);
        app.add_systems(FixedUpdate, handle_collisions);
    }
}

#[derive(Component, Debug, Copy, Clone)]
struct Ball {
    pos: Vec2,
    vel: Vec2,
    radius: f32,
}

impl Ball {
    fn intersects(&self, other: &Self) -> bool {
        let d = self.pos.distance(other.pos);
        d < self.radius + other.radius
    }

    fn mass(&self) -> f32 {
        self.radius * self.radius * 10.0
    }
}

#[derive(Component, Debug)]
struct Collision {
    e1: Entity,
    e2: Entity,
}

fn elastic_collision(b1: &Ball, b2: &Ball) -> (Vec2, Vec2) {
    let msum = b1.mass() + b2.mass();
    let dv = b2.vel - b1.vel;
    let ds = b2.pos - b1.pos;

    let v1p = b1.vel - ((2.0 * b2.mass() / msum) * dv.dot(ds) / ds.norm_squared()) * -ds;
    let v2p = b2.vel - ((2.0 * b1.mass() / msum) * dv.dot(ds) / ds.norm_squared()) * ds;

    (v1p, v2p)
}

const BALL_VELOCITY_UPPER_BOUND: f32 = 100.0;

fn setup(mut commands: Commands, window: Query<&Window>) {

    if window.is_empty() {
        return;
    }

    let h = window.single().height();
    let w = window.single().width();

    (0..100).for_each(|_| {
        let b = Ball {
            pos: Vec2::new(rand(-w / 2.0, w / 2.0), rand(-h / 2.0, h / 2.0)),
            vel: Vec2::new(rand(-1.0, 1.0), rand(-1.0, 1.0)) * BALL_VELOCITY_UPPER_BOUND,
            radius: rand(5.0, 30.0),
        };
        commands.spawn(b);
    })
}

fn draw_balls(mut gizmos: Gizmos, balls: Query<&Ball>) {
    for ball in balls.iter() {
        let iso = Isometry2d::new(ball.pos, 0.0.into());
        gizmos.circle_2d(iso, ball.radius, WHITE);
    }
}

const GRAVITY: f32 = 0.0;

fn update_balls(
    mut commands: Commands,
    time: Res<Time>,
    mut balls: Query<(Entity, &mut Ball)>,
    window: Query<&Window>,
) {
    let gravity = Vec2::new(0.0, -GRAVITY);

    if window.is_empty() {
        return;
    }

    let h = window.single().height();
    let w = window.single().width();

    let dt = time.delta().as_secs_f32();
    for (_, mut ball) in balls.iter_mut() {
        ball.vel += gravity * dt;
        let v = ball.vel;
        ball.pos += v * dt;

        if ball.pos.y - ball.radius < h / -2.0 && ball.vel.y < 0.0 {
            ball.vel.y *= -1.0;
        }

        if ball.pos.y + ball.radius > h / 2.0 && ball.vel.y > 0.0 {
            ball.vel.y *= -1.0;
        }

        if ball.pos.x - ball.radius < w / -2.0 && ball.vel.x < 0.0 {
            ball.vel.x *= -1.0;
        }

        if ball.pos.x + ball.radius > w / 2.0 && ball.vel.x > 0.0 {
            ball.vel.x *= -1.0;
        }
    }

    for (e1, b1) in balls.iter() {
        for (e2, b2) in balls.iter() {
            if e1 >= e2 {
                continue;
            }
            let ds = b2.pos - b1.pos;
            let dv = b2.vel - b1.vel;

            if b1.intersects(b2) && ds.dot(dv) < 0.0 {
                commands.spawn(Collision { e1, e2 });
            }
        }
    }
}

fn handle_collisions(
    mut command: Commands,
    mut balls: Query<&mut Ball>,
    col: Query<(Entity, &Collision)>,
) {
    for (e, c) in col.iter() {
        if let Ok(mut balls) = balls.get_many_mut([c.e1, c.e2]) {
            let (v1p, v2p) = elastic_collision(&balls[0], &balls[1]);
            balls[0].vel = v1p;
            balls[1].vel = v2p;
        }
        command.entity(e).despawn();
    }
}
