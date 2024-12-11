use crate::core::*;
use bevy::math::Vec2;
use std::time::Duration;

pub fn earth_moon_example_one() -> OrbitalSystem {
    let mut system = OrbitalSystem::default();

    let e = system.add_massive(EARTH.0, EARTH.1);
    let l = system.add_massive(LUNA.0, LUNA.1);

    for _ in 0..6 {
        system.add_massless(Propagator::Kepler(KeplerPropagator {
            epoch: Duration::default(),
            primary: e,
            orbit: Orbit {
                eccentricity: rand(0.2, 0.8),
                semi_major_axis: rand(600.0, 2600.0),
                arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                true_anomaly: rand(0.0, std::f32::consts::PI * 2.0),
                body: EARTH.0,
            },
        }));
    }

    for _ in 0..3 {
        system.add_massless(Propagator::NBody(NBodyPropagator {
            epoch: Duration::default(),
            pos: randvec(600.0, 1800.0).into(),
            vel: randvec(50.0, 100.0).into(),
        }));
    }

    for _ in 0..2 {
        system.add_massless(Propagator::Kepler(KeplerPropagator {
            epoch: Duration::default(),
            primary: l,
            orbit: Orbit {
                eccentricity: rand(0.2, 0.5),
                semi_major_axis: rand(100.0, 400.0),
                arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                true_anomaly: rand(0.0, std::f32::consts::PI * 2.0),
                body: LUNA.0,
            },
        }));
    }

    for _ in 0..4 {
        system.add_massless(Propagator::NBody(NBodyPropagator {
            epoch: Duration::default(),
            pos: randvec(7000.0, 8000.0).into(),
            vel: randvec(10.0, 15.0).into(),
        }));
    }

    system
}

pub fn n_body_stability() -> OrbitalSystem {
    let mut system: OrbitalSystem = OrbitalSystem::default();

    let e = system.add_massive(EARTH.0, EARTH.1);

    let pos = Vec2::new(7500.0, 0.0);
    let vel = Vec2::new(0.0, 15.0);

    let orbit = Orbit::from_pv(pos, vel, EARTH.0);

    system.add_massless(Propagator::Kepler(KeplerPropagator {
        epoch: Duration::default(),
        primary: e,
        orbit,
    }));

    system.add_massless(Propagator::NBody(NBodyPropagator {
        epoch: Duration::default(),
        pos,
        vel,
    }));

    system
}
