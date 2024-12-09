use std::collections::VecDeque;

use crate::util::{rand, randvec};
use bevy::color::palettes::basic::*;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

const GRAVITATIONAL_CONSTANT: f32 = 12000.0;

#[derive(Debug, Clone, Copy)]
struct KeplerOrbit {
    ecc: f32,
    sma: f32,
    argp: f32,
}

impl KeplerOrbit {
    fn from_pv(r: Vec2, v: Vec2, planet: &Planet) -> Option<Self> {
        let mu = planet.mass * GRAVITATIONAL_CONSTANT;
        let r3 = r.extend(0.0);
        let v3 = v.extend(0.0);
        let h = r3.cross(v3);
        let e = v3.cross(h) / mu - r3 / r3.length();
        let argp = f32::atan2(e.y, e.x);
        let sma = h.length_squared() / (mu * (1.0 - e.length_squared()));

        Some(KeplerOrbit {
            ecc: e.length(),
            sma,
            argp,
        })
    }

    fn radius(&self, true_anomaly: f32) -> f32 {
        self.sma * (1.0 - self.ecc.powi(2)) / (1.0 + self.ecc * f32::cos(true_anomaly))
    }

    fn evaluate(&self, true_anomaly: f32) -> Vec2 {
        let r = self.radius(true_anomaly);
        Vec2::from_angle(true_anomaly + self.argp) * r
    }
}

fn draw_orbit(orb: KeplerOrbit, gizmos: &mut Gizmos, alpha: f32) {
    if orb.ecc >= 1.0 {
        let n_points = 30;
        let theta_inf = f32::acos(-1.0 / orb.ecc);
        let points: Vec<_> = (-n_points..n_points)
            .map(|i| 0.98 * theta_inf * i as f32 / n_points as f32)
            .map(|t| orb.evaluate(t))
            .collect();
        gizmos.linestrip_2d(points, Srgba { alpha, ..RED })
    }

    let color = Srgba { alpha, ..GRAY };
    let periapsis = orb.evaluate(0.0);
    let apoapsis = orb.evaluate(std::f32::consts::PI);

    let b = orb.sma * (1.0 - orb.ecc.powi(2)).sqrt();
    let center = (periapsis + apoapsis) / 2.0;
    let iso = Isometry2d::new(center, orb.argp.into());
    gizmos
        .ellipse_2d(iso, Vec2::new(orb.sma, b), color)
        .resolution(orb.sma.clamp(3.0, 200.0) as u32);
}

#[derive(Component, Copy, Clone)]
struct Planet {
    center: Vec2,
    radius: f32,
    mass: f32,
}

#[derive(Component, Clone)]
struct Orbiter {
    pos: Vec2,
    vel: Vec2,
    on_escape: bool,
    pos_history: VecDeque<Vec2>,
}

#[derive(Component, Clone, Copy)]
struct Collision {
    pos: Vec2,
    relative_to: Entity,
    expiry_time: f32,
}

fn grav_accel(p: &Planet, c: Vec2) -> Vec2 {
    let r = p.center - c;
    let rsq = r.length_squared().clamp(p.radius.powi(2), std::f32::MAX);
    let a = GRAVITATIONAL_CONSTANT * p.mass / rsq;
    a * r.normalize()
}

impl Orbiter {
    fn random() -> Self {
        Orbiter {
            pos: randvec(70.0, 5000.0),
            vel: randvec(50.0, 500.0),
            on_escape: false,
            pos_history: VecDeque::new(),
        }
    }

    fn energy(&self, planet: &Planet) -> f32 {
        [planet]
            .iter()
            .map(|p| {
                let r = self.pos - p.center;
                -GRAVITATIONAL_CONSTANT * p.mass / r.length()
            })
            .sum::<f32>()
            + 0.5 * self.vel.length_squared()
    }

    fn contribution(&self, planets: &[&Planet]) -> f32 {
        let contr: Vec<f32> = planets
            .iter()
            .map(|p| grav_accel(p, self.pos).length())
            .collect();
        f32::min(contr[0], contr[1]) / f32::max(contr[0], contr[1])
    }

    fn step(&mut self, planets: &[&Planet], dt: f32) {
        if self.pos_history.is_empty() {
            self.pos_history.push_back(self.pos);
        } else if self.pos_history.back().unwrap().distance(self.pos) > 10.0 {
            self.pos_history.push_back(self.pos);
        }
        if self.pos_history.len() > 10 {
            self.pos_history.pop_front();
        }

        let steps = self.vel.length().clamp(2.0, 2000.0) as u32;

        (0..steps).for_each(|_| {
            let a: Vec2 = planets
                .iter()
                .map(|p| -> Vec2 { grav_accel(p, self.pos) })
                .sum();

            self.vel += a * dt / steps as f32;
            self.pos += self.vel * dt / steps as f32;
        });

        self.on_escape = self.energy(&Planet::earth()) >= 0.0;
    }
}

#[derive(Component)]
struct Moon {}

impl Planet {
    fn earth() -> Self {
        Planet {
            center: Vec2::new(0.0, 0.0),
            radius: 63.0,
            mass: 1000.0,
        }
    }

    fn luna() -> Self {
        Planet {
            center: Vec2::new(-3800.0, 200.0),
            radius: 22.0,
            mass: 10.0,
        }
    }
}

pub struct PlanetaryPlugin;

impl Plugin for PlanetaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_planet);
        app.add_systems(
            Update,
            (
                draw_orbiters,
                draw_collisions,
                draw_planets,
                keyboard_input,
                scroll_events,
            ),
        );
        app.add_systems(
            FixedUpdate,
            (
                propagate_orbiters,
                collide_orbiters,
                update_collisions,
                breed_orbiters,
                update_luna,
                despawn_escaped,
            ),
        );
    }
}

const MAX_ORBITERS: usize = 800;

#[derive(Resource)]
struct PlanetaryConfig {
    sim_speed: f32,
}

fn spawn_planet(mut commands: Commands) {
    commands.spawn(Planet::earth());
    commands.spawn((Planet::luna(), Moon {}));
    (0..MAX_ORBITERS).for_each(|_| {
        commands.spawn(Orbiter::random());
    });
    commands.insert_resource(PlanetaryConfig { sim_speed: 15.0 });
}

fn update_luna(
    time: Res<Time>,
    mut query: Query<&mut Planet, With<Moon>>,
    config: Res<PlanetaryConfig>,
    mut param: Local<f32>,
) {
    *param += config.sim_speed * time.delta().as_secs_f32();
    let r: f32 = Planet::luna().center.length();
    let two_pi = 2.0 * std::f32::consts::PI;
    let period = two_pi * (r.powi(3) / (GRAVITATIONAL_CONSTANT * Planet::earth().mass)).sqrt();
    let mut luna = query.single_mut();
    luna.center = Vec2::from_angle(two_pi * *param / period) * r;
}

fn draw_planets(mut gizmos: Gizmos, query: Query<&Planet>) {
    for planet in query.iter() {
        let iso = Isometry2d::from_translation(planet.center);
        gizmos.circle_2d(iso, planet.radius, WHITE);
    }
}

fn draw_orbiters(mut gizmos: Gizmos, query: Query<&Orbiter>, pq: Query<&Planet>) {
    let planets: Vec<&Planet> = pq.iter().collect();
    for orbiter in query.iter() {
        let contr = orbiter.contribution(&planets);
        let color = if orbiter.on_escape {
            RED
        } else {
            let b: f32 = contr.powf(0.6);
            Srgba {
                red: 1.0,
                blue: 1.0,
                green: 1.0 - b,
                alpha: 1.0,
            }
        };

        if orbiter.on_escape {
            let fadeout = Srgba {
                alpha: 0.0,
                ..color
            };

            let mut pc: Vec<(Vec2, Srgba)> = orbiter
                .pos_history
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    (
                        *p,
                        fadeout.mix(&color, i as f32 / orbiter.pos_history.len() as f32),
                    )
                })
                .collect::<Vec<_>>();
            pc.push((orbiter.pos, color));
            gizmos.linestrip_gradient_2d(pc);
        }

        let iso = Isometry2d::from_translation(orbiter.pos);
        gizmos.circle_2d(iso, 6.0, color);

        if let Some(orb) = KeplerOrbit::from_pv(orbiter.pos, orbiter.vel, planets[0]) {
            draw_orbit(orb, &mut gizmos, contr.clamp(0.02, 1.0));
        }
    }
}

fn propagate_orbiters(
    time: Res<Time>,
    mut oq: Query<&mut Orbiter>,
    pq: Query<&Planet>,
    config: Res<PlanetaryConfig>,
) {
    let dt = time.delta().as_secs_f32();

    let planets: Vec<&Planet> = pq.iter().collect();

    for mut orbiter in oq.iter_mut() {
        orbiter.step(&planets, dt * config.sim_speed);
    }
}

fn collide_orbiters(
    time: Res<Time>,
    mut commands: Commands,
    oq: Query<(Entity, &Orbiter)>,
    pq: Query<(Entity, &Planet)>,
) {
    let planets: Vec<(Entity, &Planet)> = pq.iter().collect();

    for (oe, orbiter) in oq.iter() {
        if let Some((e, p)) = planets
            .iter()
            .map(|(pe, p)| {
                let r = orbiter.pos - p.center;
                if r.length_squared() < p.radius * p.radius {
                    return Some((pe, p));
                }
                None
            })
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .next()
        {
            commands.entity(oe).despawn();
            commands.spawn(Collision {
                pos: orbiter.pos - p.center,
                relative_to: *e,
                expiry_time: time.elapsed_secs() + 3.0,
            });
        }
    }
}

fn breed_orbiters(mut commands: Commands, query: Query<&Orbiter>) {
    if query.iter().count() > MAX_ORBITERS {
        return;
    }

    for orbiter in query.iter() {
        if rand(0.0, 1.0) < 0.1 {
            let o = Orbiter {
                pos: orbiter.pos,
                vel: Vec2::from_angle(rand(-0.12, 0.12)).rotate(orbiter.vel),
                on_escape: false,
                pos_history: VecDeque::new(),
            };
            commands.spawn(o);
        }
    }
}

fn draw_collisions(mut gizmos: Gizmos, query: Query<&Collision>, planets: Query<&Planet>) {
    for col in query.iter() {
        if let Some(p) = planets.get(col.relative_to).ok() {
            let s = 9.0;
            let ne = Vec2::new(s, s);
            let nw = Vec2::new(-s, s);
            let p = p.center + col.pos;
            gizmos.line_2d(p - ne, p + ne, RED);
            gizmos.line_2d(p - nw, p + nw, RED);
        }
    }
}

fn update_collisions(mut commands: Commands, time: Res<Time>, query: Query<(Entity, &Collision)>) {
    let t = time.elapsed_secs();
    for (e, col) in query.iter() {
        if col.expiry_time < t {
            commands.entity(e).despawn();
        }
    }
}

fn despawn_escaped(mut commands: Commands, query: Query<(Entity, &Orbiter)>) {
    for (e, orbiter) in query.iter() {
        if orbiter.on_escape && orbiter.pos.length() > 10000.0 {
            commands.entity(e).despawn();
        }
    }
}

fn keyboard_input(keys: Res<ButtonInput<KeyCode>>, mut config: ResMut<PlanetaryConfig>) {
    for key in keys.get_pressed() {
        match key {
            KeyCode::ArrowDown | KeyCode::KeyS => {
                config.sim_speed = f32::clamp(config.sim_speed - 0.05, 0.0, 20.0);
                dbg!(config.sim_speed);
            }
            KeyCode::ArrowUp | KeyCode::KeyW => {
                config.sim_speed = f32::clamp(config.sim_speed + 0.05, 0.0, 20.0);
                dbg!(config.sim_speed);
            }
            KeyCode::ArrowLeft | KeyCode::KeyA => {
                config.sim_speed = f32::clamp(config.sim_speed - 1.0, 0.0, 20.0);
                dbg!(config.sim_speed);
            }
            KeyCode::ArrowRight | KeyCode::KeyD => {
                config.sim_speed = f32::clamp(config.sim_speed + 1.0, 0.0, 20.0);
                dbg!(config.sim_speed);
            }
            _ => (),
        }
    }
}

fn scroll_events(mut evr_scroll: EventReader<MouseWheel>, mut transforms: Query<&mut Transform, With<Camera>>) {
    use bevy::input::mouse::MouseScrollUnit;

    let mut transform = transforms.single_mut();

    for ev in evr_scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                if ev.y > 0.0
                {
                    transform.scale /= 1.1;
                }
                else
                {
                    transform.scale *= 1.1;
                }
            },
            _ => ()
        }
    }
}
