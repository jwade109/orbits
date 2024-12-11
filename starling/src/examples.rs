use crate::core::*;
use std::time::Duration;

use bevy_ecs::entity::Entity;

pub fn earth_moon_example_one() -> OrbitalSystem {
    let mut system = OrbitalSystem::default();

    let e = system.add_object(ObjectClass::Body(EARTH.0), EARTH.1);
    let l = system.add_object(ObjectClass::Body(LUNA.0), LUNA.1);

    for _ in 0..6 {
        system.add_object(
            ObjectClass::Orbiter,
            Propagator::Kepler(KeplerPropagator {
                epoch: Duration::default(),
                primary: Entity::from_raw(0),
                orbit: Orbit {
                    eccentricity: rand(0.2, 0.8),
                    semi_major_axis: rand(600.0, 2600.0),
                    arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                    true_anomaly: rand(0.0, std::f32::consts::PI * 2.0),
                    body: EARTH.0,
                },
            }),
        );
    }

    for _ in 0..3 {
        system.add_object(
            ObjectClass::Orbiter,
            Propagator::NBody(NBodyPropagator {
                epoch: Duration::default(),
                pos: randvec(600.0, 1800.0).into(),
                vel: randvec(50.0, 100.0).into(),
            }),
        );
    }

    for _ in 0..2 {
        system.add_object(
            ObjectClass::Orbiter,
            Propagator::Kepler(KeplerPropagator {
                epoch: Duration::default(),
                primary: Entity::from_raw(0),
                orbit: Orbit {
                    eccentricity: rand(0.2, 0.5),
                    semi_major_axis: rand(100.0, 400.0),
                    arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                    true_anomaly: rand(0.0, std::f32::consts::PI * 2.0),
                    body: LUNA.0,
                },
            }),
        );
    }

    system
}
