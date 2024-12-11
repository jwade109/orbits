use crate::util::{rand, randvec};
use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::{LIGHT_BLUE, ORANGE};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use std::time::Duration;

use starling::*;

// ORBIT PROPAGATION AND N-BODY SIMULATION

#[derive(Resource, Default)]
struct SimTime(Duration);

#[derive(Component, Copy, Clone, Debug)]
struct BodyC(Body);

#[derive(Component, Clone)]
struct Orbiter {}

#[derive(Component, Clone, Copy)]
struct Collision {
    pos: Vec2,
    relative_to: Entity,
    expiry_time: f32,
}

// DRAWING STUFF

fn draw_orbit(origin: Vec2, orb: Orbit, gizmos: &mut Gizmos, alpha: f32, base_color: Srgba) {
    if orb.eccentricity >= 1.0 {
        let n_points = 30;
        let theta_inf = f32::acos(-1.0 / orb.eccentricity);
        let points: Vec<_> = (-n_points..n_points)
            .map(|i| 0.98 * theta_inf * i as f32 / n_points as f32)
            .map(|t| orb.position_at(t))
            .collect();
        gizmos.linestrip_2d(points, Srgba { alpha, ..RED })
    }

    let color = Srgba {
        alpha,
        ..base_color
    };

    let b = orb.semi_major_axis * (1.0 - orb.eccentricity.powi(2)).sqrt();
    let center: Vec2 = origin + (orb.periapsis() + orb.apoapsis()) / 2.0;
    let iso = Isometry2d::new(center, orb.arg_periapsis.into());
    gizmos
        .ellipse_2d(iso, Vec2::new(orb.semi_major_axis, b), color)
        .resolution(orb.semi_major_axis.clamp(3.0, 200.0) as u32);

    let line_start = origin + orb.pos().normalize() * (orb.body.radius + 5.0);

    gizmos.line_2d(line_start, origin + orb.pos(), color);
    gizmos.circle_2d(
        Isometry2d::from_translation(origin + orb.periapsis()),
        2.0,
        Srgba { alpha, ..RED },
    );
    gizmos.circle_2d(
        Isometry2d::from_translation(origin + orb.apoapsis()),
        2.0,
        Srgba { alpha, ..WHITE },
    );
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
                draw_all_propagators,
            ),
        );
        app.add_systems(
            FixedUpdate,
            (
                propagate_bodies,
                propagate_orbiters,
                collide_orbiters,
                update_collisions,
                despawn_escaped,
                update_sim_time,
            ),
        );
    }
}

const MAX_ORBITERS: usize = 10;

#[derive(Resource)]
struct PlanetaryConfig {
    sim_speed: f32,
    show_orbits: bool,
    paused: bool,
}

impl Default for PlanetaryConfig {
    fn default() -> Self {
        PlanetaryConfig {
            sim_speed: 15.0,
            show_orbits: true,
            paused: false,
        }
    }
}

fn to_bundle(b: (Body, Propagator)) -> (BodyC, PropagatorC) {
    (BodyC(b.0), PropagatorC(b.1))
}

fn spawn_planet(mut commands: Commands) {
    commands.insert_resource(PlanetaryConfig::default());
    commands.insert_resource(SimTime::default());
    let e = commands.spawn(to_bundle(Body::earth())).id();
    let l = commands.spawn(to_bundle(Body::luna())).id();

    for _ in 0..6 {
        commands.spawn((
            Orbiter {},
            PropagatorC(Propagator::Kepler(KeplerPropagator {
                epoch: Duration::default(),
                primary: e,
                orbit: Orbit {
                    eccentricity: rand(0.2, 0.8),
                    semi_major_axis: rand(600.0, 2600.0),
                    arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                    true_anomaly: rand(0.0, std::f32::consts::PI * 2.0),
                    body: Body::earth().0,
                },
            })),
        ));
    }

    for _ in 0..3 {
        commands.spawn((
            Orbiter {},
            PropagatorC(Propagator::NBody(NBodyPropagator {
                epoch: Duration::default(),
                pos: randvec(600.0, 1800.0).into(),
                vel: randvec(50.0, 100.0).into(),
            })),
        ));
    }

    for _ in 0..2 {
        commands.spawn((
            Orbiter {},
            PropagatorC(Propagator::Kepler(KeplerPropagator {
                epoch: Duration::default(),
                primary: l,
                orbit: Orbit {
                    eccentricity: rand(0.2, 0.5),
                    semi_major_axis: rand(100.0, 400.0),
                    arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                    true_anomaly: rand(0.0, std::f32::consts::PI * 2.0),
                    body: Body::luna().0,
                },
            })),
        ));
    }
}

fn draw_planets(mut gizmos: Gizmos, query: Query<(&PropagatorC, &BodyC)>) {
    for (PropagatorC(prop), BodyC(body)) in query.iter() {
        let iso = Isometry2d::from_translation(prop.pos());
        gizmos.circle_2d(iso, body.radius, WHITE);
        gizmos.circle_2d(
            iso,
            body.soi,
            Srgba {
                alpha: 0.3,
                ..ORANGE
            },
        );

        let orb: Orbit = Orbit::from_pv(prop.pos(), prop.vel(), Body::earth().0);
        draw_orbit((0.0, 0.0).into(), orb, &mut gizmos, 0.6, GRAY);
    }
}

fn draw_all_propagators(mut gizmos: Gizmos, query: Query<&PropagatorC>) {
    for PropagatorC(prop) in query.iter() {
        let pos: Option<Vec2> = match prop {
            Propagator::Fixed(p) => Some(*p),
            Propagator::NBody(nb) => Some(nb.pos),
            Propagator::Kepler(k) => query
                .get(k.primary)
                .ok()
                .map(|PropagatorC(p)| prop.pos() + p.pos()),
        };

        if let Some(p) = pos {
            gizmos.circle_2d(
                Isometry2d::from_translation(p),
                120.0,
                Srgba { alpha: 0.05, ..RED },
            );
        }
    }
}

fn draw_orbiters(
    mut gizmos: Gizmos,
    query: Query<&PropagatorC, With<Orbiter>>,
    pq: Query<(&BodyC, &PropagatorC)>,
    config: Res<PlanetaryConfig>,
) {
    for PropagatorC(prop) in query.iter() {
        match prop {
            Propagator::Fixed(p) => {
                let color: Srgba = RED;
                let iso: Isometry2d = Isometry2d::from_translation(*p);
                gizmos.circle_2d(iso, 20.0, color);
            }
            Propagator::NBody(nb) => {
                let color: Srgba = WHITE;
                let iso: Isometry2d = Isometry2d::from_translation(nb.pos);
                gizmos.circle_2d(iso, 12.0, color);
                if config.show_orbits {
                    let orb = Orbit::from_pv(nb.pos, nb.vel, Body::earth().0);
                    draw_orbit(Vec2::ZERO, orb, &mut gizmos, 0.2, LIGHT_BLUE);
                }
            }
            Propagator::Kepler(k) => {
                if let Some((_, PropagatorC(parent))) = pq.get(k.primary).ok() {
                    let color: Srgba = ORANGE;
                    let iso: Isometry2d = Isometry2d::from_translation(prop.pos() + parent.pos());
                    gizmos.circle_2d(iso, 12.0, color);
                    if config.show_orbits {
                        draw_orbit(parent.pos(), k.orbit, &mut gizmos, 0.02, WHITE);
                    }
                }
            }
        }
    }
}

#[derive(Component, Debug, Copy, Clone)]
struct PropagatorC(Propagator);

fn update_sim_time(time: Res<Time>, mut simtime: ResMut<SimTime>, config: Res<PlanetaryConfig>) {
    if config.paused {
        return;
    }
    let SimTime(dur) = simtime.as_mut();
    *dur = *dur + Duration::from_nanos((time.delta().as_nanos() as f32 * config.sim_speed) as u64);
}

fn propagate_orbiters(
    time: Res<SimTime>,
    mut pq: Query<&mut PropagatorC, Without<BodyC>>,
    bq: Query<(&PropagatorC, &BodyC)>,
) {
    let SimTime(t) = *time;
    let bodies: Vec<(Vec2, Body)> = bq
        .into_iter()
        .map(|(PropagatorC(p), BodyC(b))| (p.pos(), *b))
        .collect();

    pq.iter_mut().for_each(|mut p| {
        let PropagatorC(prop) = p.as_mut();
        match prop {
            Propagator::Kepler(k) => {
                k.propagate_to(t);
            }
            Propagator::NBody(nb) => {
                nb.propagate_to(&bodies, t);
            }
            &mut Propagator::Fixed(_) => (),
        }
    });
}

fn propagate_bodies(time: Res<SimTime>, mut query: Query<(&mut PropagatorC, &BodyC)>) {
    let SimTime(t) = *time;

    let bodies: Vec<(Vec2, Body)> = query
        .into_iter()
        .map(|(PropagatorC(p), BodyC(b))| (p.pos(), *b))
        .collect();

    query.iter_mut().for_each(|(mut p, _)| {
        let PropagatorC(current_prop) = p.as_mut();
        let other_bodies = bodies
            .clone()
            .into_iter()
            .filter(|(p, _)| p.distance(current_prop.pos()) > 0.0)
            .collect::<Vec<_>>();

        match current_prop {
            Propagator::NBody(nb) => nb.propagate_to(&other_bodies, t),
            Propagator::Fixed(_) => (),
            Propagator::Kepler(k) => k.propagate_to(t),
        }
    })
}

fn collide_orbiters(
    time: Res<Time>,
    mut commands: Commands,
    oq: Query<(Entity, &PropagatorC), With<Orbiter>>,
    pq: Query<(Entity, &PropagatorC, &BodyC), Without<Orbiter>>,
) {
    let bodies: Vec<(Entity, Vec2, Body)> = pq
        .into_iter()
        .map(|(e, PropagatorC(p), BodyC(b))| (e, p.pos(), *b))
        .collect();

    for (oe, PropagatorC(prop)) in oq.iter() {
        if let Some((hit_entity, hit_entity_pos)) = bodies
            .iter()
            .map(|(pe, c, b)| {
                let r = prop.pos() - c;
                if r.length_squared() < b.radius * b.radius {
                    return Some((pe, c));
                }
                None
            })
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .next()
        {
            commands.entity(oe).despawn();
            commands.spawn(Collision {
                pos: prop.pos() - hit_entity_pos,
                relative_to: *hit_entity,
                expiry_time: time.elapsed_secs() + 3.0,
            });
        }
    }
}

fn draw_collisions(
    mut gizmos: Gizmos,
    query: Query<&Collision>,
    planets: Query<&PropagatorC, With<BodyC>>,
) {
    for col in query.iter() {
        if let Some(PropagatorC(p)) = planets.get(col.relative_to).ok() {
            let s = 9.0;
            let ne = Vec2::new(s, s);
            let nw = Vec2::new(-s, s);
            let p = p.pos() + col.pos;
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

fn despawn_escaped(mut commands: Commands, query: Query<(Entity, &PropagatorC), With<Orbiter>>) {
    for (e, PropagatorC(prop)) in query.iter() {
        if prop.pos().length() > 15000.0 {
            commands.entity(e).despawn();
        }
    }
}

fn keyboard_input(keys: Res<ButtonInput<KeyCode>>, mut config: ResMut<PlanetaryConfig>) {
    for key in keys.get_pressed() {
        match key {
            KeyCode::ArrowDown => {
                config.sim_speed = f32::clamp(config.sim_speed - 0.01, 0.0, 1200.0);
                dbg!(config.sim_speed);
            }
            KeyCode::ArrowUp => {
                config.sim_speed = f32::clamp(config.sim_speed + 0.01, 0.0, 1200.0);
                dbg!(config.sim_speed);
            }
            KeyCode::ArrowLeft => {
                config.sim_speed = f32::clamp(config.sim_speed - 1.0, 0.0, 1200.0);
                dbg!(config.sim_speed);
            }
            KeyCode::ArrowRight => {
                config.sim_speed = f32::clamp(config.sim_speed + 1.0, 0.0, 1200.0);
                dbg!(config.sim_speed);
            }
            KeyCode::Period => {
                config.sim_speed = 1.0;
                dbg!(config.sim_speed);
            }
            _ => {
                dbg!(key);
            }
        }
    }

    if keys.just_pressed(KeyCode::KeyO) {
        config.show_orbits = !config.show_orbits;
    }
    if keys.just_pressed(KeyCode::Space) {
        config.paused = !config.paused;
    }
}

fn scroll_events(
    mut evr_scroll: EventReader<MouseWheel>,
    mut transforms: Query<&mut Transform, With<Camera>>,
) {
    use bevy::input::mouse::MouseScrollUnit;

    let mut transform = transforms.single_mut();

    for ev in evr_scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                if ev.y > 0.0 {
                    transform.scale /= 1.1;
                } else {
                    transform.scale *= 1.1;
                }
            }
            _ => (),
        }
    }
}
