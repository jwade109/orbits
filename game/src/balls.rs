use bevy::prelude::*;
use bevy::color::palettes::basic::*;
use rand::Rng;

pub struct BallsPlugin;

fn rand(min: f32, max: f32) -> f32 {
    rand::thread_rng().gen_range(min..max)
}

impl Plugin for BallsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, draw);
        app.add_systems(FixedUpdate, update_balls);
        app.add_systems(FixedUpdate, despawn_collisions);
    }
}

#[derive(Component, Debug, Copy, Clone)]
struct Ball {
    pos: Vec2,
    vel: Vec2,
    radius: f32,
}

impl Ball {
    fn intersects(self: Self, other: Self) -> bool {
        let d = self.pos.distance(other.pos);
        d < self.radius + other.radius
    }
}

#[derive(Component, Debug)]
struct Collision {
    e1: Entity,
    e2: Entity,
}

fn get_balls<'a>(col: &Collision, balls: &Query<&Ball>) -> Option<(Ball, Ball)> {
    let b1 = balls.get(col.e1).ok();
    let b2 = balls.get(col.e2).ok();

    match (b1, b2) {
        (Some(x), Some(y)) => Some((*x, *y)),
        _ => None,
    }
}

fn setup(mut commands: Commands, window: Query<&Window>) {
    commands.spawn(Camera2d);

    if window.is_empty() {
        return;
    }

    let h = window.single().height();
    let w = window.single().width();

    (0..50).for_each(|_| {
        let b = Ball {
            pos: Vec2::new(rand(-w / 2.0, w / 2.0), rand(-h / 2.0, h / 2.0)),
            vel: Vec2::new(rand(-300.0, 300.0), rand(-300.0, 300.0)),
            radius: rand(5.0, 40.0),
        };
        commands.spawn(b);
    })
}

fn draw(mut gizmos: Gizmos, balls: Query<&Ball>, col: Query<&Collision>) {
    for ball in balls.iter() {
        let iso = Isometry2d::new(ball.pos, 0.0.into());
        gizmos.circle_2d(iso, ball.radius, WHITE);
    }
    for c in col.iter() {
        let p1 = if let Ok(b1) = balls.get(c.e1) {
            b1.pos
        } else {
            continue;
        };

        let p2 = if let Ok(b2) = balls.get(c.e2) {
            b2.pos
        } else {
            continue;
        };

        gizmos.line_2d(p1, p2, RED);
    }
}

const GRAVITY : f32 = 0.0;

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
            let d = b1.pos.distance(b2.pos);
            if d < b1.radius + b2.radius && d > 0.0 {
                commands.spawn(Collision { e1, e2 });
            }
        }
    }
}

fn despawn_collisions(
    mut command: Commands,
    balls: Query<&Ball>,
    col: Query<(Entity, &Collision)>,
) {
    for (e, c) in col.iter() {
        if let Some((b1, b2)) = get_balls(c, &balls) {
            if !b1.intersects(b2) {
                command.entity(e).despawn();
            }
        }
    }
}
