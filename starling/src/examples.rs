use crate::core::*;
use crate::orbit::*;
use crate::propagator::*;
use bevy::math::Vec2;
use std::time::Duration;

#[cfg(test)]
use approx::assert_relative_eq;

pub const EARTH: (Body, Vec2) = (
    Body {
        radius: 63.0,
        mass: 1000.0,
        soi: 15000.0,
    },
    Vec2::ZERO,
);

pub const LUNA: (Body, NBodyPropagator) = (
    Body {
        radius: 22.0,
        mass: 10.0,
        soi: 800.0,
    },
    NBodyPropagator {
        epoch: Duration::new(0, 0),
        dt: Duration::from_millis(100),
        pos: Vec2::new(-3800.0, 0.0),
        vel: Vec2::new(0.0, -58.0),
    },
);

pub fn earth_moon_example_one() -> OrbitalSystem {
    let mut system = OrbitalSystem::default();

    let e = system.add_object(EARTH.1, Some(EARTH.0));
    let l = system.add_object(LUNA.1, Some(LUNA.0));

    for _ in 0..4 {
        system.add_object(
            KeplerPropagator::new(
                Orbit {
                    eccentricity: rand(0.2, 0.8),
                    semi_major_axis: rand(600.0, 2600.0),
                    arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                    true_anomaly: rand(0.0, std::f32::consts::PI * 2.0),
                    retrograde: rand(0.0, 1.0) < 0.3,
                    body: EARTH.0,
                },
                e,
            ),
            None,
        );
    }

    for _ in 0..60 {
        system.add_object(
            NBodyPropagator::initial(randvec(600.0, 1800.0), randvec(80.0, 120.0)),
            None,
        );
    }

    for _ in 0..5 {
        system.add_object(
            KeplerPropagator::new(
                Orbit {
                    eccentricity: rand(0.2, 0.5),
                    semi_major_axis: rand(100.0, 400.0),
                    arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                    true_anomaly: rand(0.0, std::f32::consts::PI * 2.0),
                    retrograde: rand(0.0, 1.0) < 0.3,
                    body: LUNA.0,
                },
                l,
            ),
            None,
        );
    }

    for _ in 0..8 {
        system.add_object(
            NBodyPropagator::initial(randvec(7000.0, 8000.0), randvec(10.0, 15.0)),
            None,
        );
    }

    system.add_object(
        NBodyPropagator::initial((7500.0, 3000.0), (30.0, -10.0)),
        Some(Body::new(10.0, 2.5, 300.0)),
    );

    system.add_object(
        NBodyPropagator::initial((7500.0, 2920.0), (48.0, -10.0)),
        None,
    );

    system
}

pub fn n_body_stability() -> OrbitalSystem {
    let mut system: OrbitalSystem = OrbitalSystem::default();

    let origin = Vec2::new(-2000.0, 0.0);
    let e = system.add_object(origin, Some(EARTH.0));

    let mut add_test = |p: Vec2, v: Vec2| {
        let orbit = Orbit::from_pv(p, v, EARTH.0);

        system.add_object(KeplerPropagator::new(orbit, e), None);
        system.add_object(NBodyPropagator::initial(origin + p, v), None);
    };

    add_test((7500.0, 0.0).into(), (0.0, 7.0).into());
    add_test((6000.0, 0.0).into(), (0.0, 12.0).into());
    add_test((5000.0, 0.0).into(), (0.0, 20.0).into());
    add_test((3000.0, 0.0).into(), (0.0, 40.0).into());
    add_test((700.0, 0.0).into(), (0.0, 60.0).into());

    system
}

#[test]
pub fn n_body_accuracy() {
    let mut system = n_body_stability();

    let mut events = vec![];
    // roughly one orbital period for test bodies
    while system.epoch < Duration::from_secs(425) {
        events.extend(system.step());
    }

    assert_eq!(events.len(), 0);

    let frame = system.frame();

    let (_, pv1, _) = frame.lookup(ObjectId(1)).unwrap();
    let (_, pv2, _) = frame.lookup(ObjectId(2)).unwrap();

    assert_relative_eq!(pv1.pos.distance(pv2.pos), 2.023654, max_relative = 1.0);
}

pub fn simple_two_body() -> OrbitalSystem {
    let mut system = OrbitalSystem::default();

    let b = Some(Body {
        mass: 500.0,
        radius: 50.0,
        soi: 10000.0,
    });

    system.add_object(NBodyPropagator::initial((400.0, 0.0), (0.0, 40.0)), b);
    system.add_object(NBodyPropagator::initial((-400.0, 0.0), (0.0, -40.0)), b);

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

    let s = system.add_object(Vec2::ZERO, Some(sun));

    system.add_object(KeplerPropagator::new(jupiter_orbit, s), Some(jupiter));

    for _ in 0..600 {
        let r = randvec(4000.0, 6000.0);
        let v = Vec2::from_angle(std::f32::consts::PI / 2.0).rotate(r.normalize())
            * jupiter_orbit.pv().vel.length();
        system.add_object(NBodyPropagator::initial(r, v), None);
    }

    system
}

pub fn patched_conics_scenario() -> OrbitalSystem {
    let mut system = OrbitalSystem::default();

    let e = system.add_object(EARTH.1, Some(EARTH.0));

    system.add_object(
        KeplerPropagator::new(Orbit::circular(5000.0, 0.0, EARTH.0), e),
        Some(LUNA.0),
    );

    for _ in 0..30 {
        let r = randvec(200.0, 201.0);
        let v = Vec2::from_angle(std::f32::consts::PI / 2.0)
            .rotate(r)
            .normalize()
            * 340.0;
        system.add_object(
            KeplerPropagator::new(Orbit::from_pv(r, v, EARTH.0), e),
            None,
        );
        system.add_object(NBodyPropagator::initial(r, v), None);
    }

    system
}

pub fn playground() -> OrbitalSystem {
    let mut system = OrbitalSystem::default();

    system.add_object(Vec2::ZERO, Some(EARTH.0));

    system.add_object(Vec2::new(400.0, 300.0), Some(EARTH.0));

    system.add_object(Vec2::new(-500.0, 200.0), Some(EARTH.0));

    system.add_object(NBodyPropagator::initial((0.0, -500.0), (130.0, 0.0)), None);

    system
}

pub fn default_example() -> OrbitalSystem {
    earth_moon_example_one()
}
