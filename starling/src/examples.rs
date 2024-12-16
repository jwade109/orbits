use crate::core::*;
use bevy::math::Vec2;
use std::time::Duration;

#[cfg(test)]
use approx::assert_relative_eq;

pub fn earth_moon_example_one() -> OrbitalSystem {
    let mut system = OrbitalSystem::default();

    let e = system.add_object(EARTH.1, Some(EARTH.0));
    let l = system.add_object(LUNA.1, Some(LUNA.0));

    for _ in 0..4 {
        system.add_object(
            Propagator::Kepler(KeplerPropagator {
                epoch: Duration::default(),
                primary: e,
                orbit: Orbit {
                    eccentricity: rand(0.2, 0.8),
                    semi_major_axis: rand(600.0, 2600.0),
                    arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                    true_anomaly: rand(0.0, std::f32::consts::PI * 2.0),
                    retrograde: rand(0.0, 1.0) < 0.3,
                    body: EARTH.0,
                },
            }),
            None,
        );
    }

    for _ in 0..60 {
        system.add_object(
            Propagator::NBody(NBodyPropagator {
                epoch: Duration::default(),
                pos: randvec(600.0, 1800.0).into(),
                vel: randvec(80.0, 120.0).into(),
                history: vec![],
            }),
            None,
        );
    }

    for _ in 0..5 {
        system.add_object(
            Propagator::Kepler(KeplerPropagator {
                epoch: Duration::default(),
                primary: l,
                orbit: Orbit {
                    eccentricity: rand(0.2, 0.5),
                    semi_major_axis: rand(100.0, 400.0),
                    arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                    true_anomaly: rand(0.0, std::f32::consts::PI * 2.0),
                    retrograde: rand(0.0, 1.0) < 0.3,
                    body: LUNA.0,
                },
            }),
            None,
        );
    }

    for _ in 0..8 {
        system.add_object(
            Propagator::NBody(NBodyPropagator {
                epoch: Duration::default(),
                pos: randvec(7000.0, 8000.0).into(),
                vel: randvec(10.0, 15.0).into(),
                history: vec![],
            }),
            None,
        );
    }

    system.add_object(
        Propagator::nbody(
            Duration::default(),
            (7500.0, 3000.0).into(),
            (30.0, -10.0).into(),
        ),
        Some(Body {
            radius: 10.0,
            mass: 2.5,
            soi: 300.0,
        }),
    );

    system.add_object(
        Propagator::nbody(
            Duration::default(),
            (7500.0, 2920.0).into(),
            (48.0, -10.0).into(),
        ),
        None,
    );

    // system.add_object(Propagator::Fixed((100.0, 100.0).into(), Some(l)), None);

    system
}

pub fn n_body_stability() -> OrbitalSystem {
    let mut system: OrbitalSystem = OrbitalSystem::default();

    let e = system.add_object(EARTH.1, Some(EARTH.0));

    let pos = Vec2::new(7500.0, 0.0);
    let vel = Vec2::new(0.0, 7.0);

    let orbit = Orbit::from_pv(pos, vel, EARTH.0);

    system.add_object(
        Propagator::Kepler(KeplerPropagator {
            epoch: Duration::default(),
            primary: e,
            orbit,
        }),
        None,
    );

    system.add_object(
        Propagator::NBody(NBodyPropagator {
            epoch: Duration::default(),
            pos,
            vel,
            history: vec![],
        }),
        None,
    );

    system
}

#[test]
pub fn n_body_accuracy() {
    let mut system = n_body_stability();

    // roughly one orbital period for test bodies
    let events = system.propagate_to(Duration::from_secs(425));

    dbg!(&events);
    assert_eq!(events.len(), 0);

    let pv1 = system.transform_from_id(Some(ObjectId(1))).unwrap();

    let pv2 = system.transform_from_id(Some(ObjectId(2))).unwrap();

    assert_relative_eq!(pv1.pos.distance(pv2.pos), 285.06125);
}

pub fn simple_two_body() -> OrbitalSystem {
    let mut system = OrbitalSystem::default();

    let body = Body {
        mass: 500.0,
        radius: 50.0,
        soi: 10000.0,
    };

    system.add_object(
        Propagator::nbody(Duration::default(), (400.0, 0.0).into(), (0.0, 40.0).into()),
        Some(body),
    );
    system.add_object(
        Propagator::nbody(
            Duration::default(),
            (-400.0, 0.0).into(),
            (0.0, -40.0).into(),
        ),
        Some(body),
    );

    system
}

pub fn sun_jupiter_lagrange() -> OrbitalSystem {
    let mut system = OrbitalSystem::default();

    let sun = Body {
        mass: 1000.0,
        radius: 100.0,
        soi: 100000.0,
    };

    let jupiter = Body {
        mass: sun.mass * 0.000954588,
        radius: 20.0,
        soi: 500.0,
    };

    let jupiter_orbit = Orbit {
        eccentricity: 0.0,
        arg_periapsis: 0.0,
        semi_major_axis: 5000.0,
        body: sun,
        true_anomaly: 0.0,
        retrograde: false,
    };

    let s = system.add_object(Propagator::fixed_at(Vec2::ZERO), Some(sun));

    system.add_object(
        Propagator::kepler(Duration::default(), jupiter_orbit, s),
        Some(jupiter),
    );

    for _ in 0..600 {
        let r = randvec(4000.0, 6000.0);
        let v = Vec2::from_angle(std::f32::consts::PI / 2.0).rotate(r.normalize()) * jupiter_orbit.vel().length();
        let prop = Propagator::nbody(Duration::default(), r, v);
        system.add_object(prop, None);
    }

    // let l4 = Vec2::from_angle(std::f32::consts::PI / 3.0).rotate(jupiter_orbit.pos());
    // let l5 = Vec2::from_angle(-std::f32::consts::PI / 3.0).rotate(jupiter_orbit.pos());
    // let l4v = Vec2::from_angle(std::f32::consts::PI / 3.0).rotate(jupiter_orbit.vel());
    // let l5v = Vec2::from_angle(-std::f32::consts::PI / 3.0).rotate(jupiter_orbit.vel());

    // for _ in 0..100 {
    //     let r = l4 + randvec(0.0, 10.0);
    //     let v = l4v + randvec(0.0, 1.0);
    //     let prop = Propagator::nbody(Duration::default(), r, v);
    //     system.add_object(prop, None);
    // }

    // for _ in 0..100 {
    //     let r = l5 + randvec(0.0, 10.0);
    //     let v = l5v + randvec(0.0, 1.0);
    //     let prop = Propagator::nbody(Duration::default(), r, v);
    //     system.add_object(prop, None);
    // }

    system
}

pub fn default_example() -> OrbitalSystem {
    earth_moon_example_one()
}
