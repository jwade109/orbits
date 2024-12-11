use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::ORANGE;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use std::time::Duration;

use crate::debug::DebugLog;

use starling::core::*;
use starling::examples::*;

pub struct PlanetaryPlugin;

impl Plugin for PlanetaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);
        app.add_systems(Update, (draw_orbital_system, keyboard_input, scroll_events));
        app.add_systems(
            FixedUpdate,
            (propagate_system, update_collisions, update_sim_time),
        );
    }
}

#[derive(Debug, Resource, Default)]
struct SimTime(Duration);

#[derive(Component, Clone, Copy)]
struct Collision {
    pos: Vec2,
    relative_to: Entity,
    expiry_time: f32,
}

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

    {
        let root = orb.pos() + origin;
        let t1 = root + orb.normal() * 60.0;
        let t2 = root + orb.tangent() * 60.0;
        let t3 = root + orb.prograde() * 100.0;
        gizmos.line_2d(root, t1, GREEN);
        gizmos.line_2d(root, t2, GREEN);
        gizmos.line_2d(root, t3, PURPLE);
    }

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

#[derive(Resource)]
struct PlanetaryState {
    sim_speed: f32,
    show_orbits: bool,
    paused: bool,
    system: OrbitalSystem,
}

impl Default for PlanetaryState {
    fn default() -> Self {
        PlanetaryState {
            sim_speed: 15.0,
            show_orbits: true,
            paused: false,
            system: earth_moon_example_one(),
        }
    }
}

fn init_system(mut commands: Commands) {
    commands.insert_resource(PlanetaryState::default());
    commands.insert_resource(SimTime::default());
}

fn draw_orbital_system(mut gizmos: Gizmos, state: Res<PlanetaryState>) {
    for object in state.system.objects.iter() {
        if let Some(body) = object.body
        {
            let iso = Isometry2d::from_translation(object.prop.pos());
            gizmos.circle_2d(iso, body.radius, WHITE);
            gizmos.circle_2d(
                iso,
                body.soi,
                Srgba {
                    alpha: 0.3,
                    ..ORANGE
                },
            );

            let orb: Orbit = Orbit::from_pv(object.prop.pos(), object.prop.vel(), EARTH.0);
            draw_orbit((0.0, 0.0).into(), orb, &mut gizmos, 0.6, GRAY);
        }
    }

    for object in state.system.objects.iter() {

        match object.prop {
            Propagator::Fixed(_, _) => {
                let color: Srgba = RED;
                if let Some(gp) = state.system.global_pos(object.prop)
                {
                    let iso: Isometry2d = Isometry2d::from_translation(gp);
                    gizmos.circle_2d(iso, 20.0, color);
                }
            }
            Propagator::NBody(nb) => {
                let color: Srgba = WHITE;
                let iso: Isometry2d = Isometry2d::from_translation(nb.pos);
                gizmos.circle_2d(iso, 12.0, color);
                if state.show_orbits {
                    let orb = Orbit::from_pv(nb.pos, nb.vel, EARTH.0);
                    draw_orbit(Vec2::ZERO, orb, &mut gizmos, 0.05, WHITE);
                }
            }
            Propagator::Kepler(k) => {
                if let Some(parent) = state.system.lookup(k.primary) {
                    let color: Srgba = ORANGE;
                    let iso: Isometry2d =
                        Isometry2d::from_translation(object.prop.pos() + parent.prop.pos());
                    gizmos.circle_2d(iso, 12.0, color);
                    if state.show_orbits {
                        draw_orbit(parent.prop.pos(), k.orbit, &mut gizmos, 0.05, WHITE);
                    }
                }
            }
        }
    }

}

fn update_sim_time(time: Res<Time>, mut simtime: ResMut<SimTime>, config: Res<PlanetaryState>) {
    if config.paused {
        return;
    }
    let SimTime(dur) = simtime.as_mut();
    *dur = *dur + Duration::from_nanos((time.delta().as_nanos() as f32 * config.sim_speed) as u64);
}

fn propagate_system(time: Res<SimTime>, mut state: ResMut<PlanetaryState>) {
    let SimTime(t) = *time;
    state.system.propagate_to(t);
}

fn update_collisions(mut commands: Commands, time: Res<Time>, query: Query<(Entity, &Collision)>) {
    let t = time.elapsed_secs();
    for (e, col) in query.iter() {
        if col.expiry_time < t {
            commands.entity(e).despawn();
        }
    }
}

fn keyboard_input(keys: Res<ButtonInput<KeyCode>>, mut config: ResMut<PlanetaryState>) {
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
