use crate::core::*;
use crate::orbit::*;
use bevy::math::Vec2;

#[cfg(test)]
use approx::assert_relative_eq;

pub const EARTH: Body = Body::new(63.0, 1000.0, 15000.0);

pub const LUNA: (Body, Orbit) = (
    Body::new(22.0, 10.0, 800.0),
    Orbit::circular(3800.0, EARTH.mass, Nanotime(-40 * 1000000000), false),
);

pub fn just_the_moon() -> OrbitalSystem {
    let mut subsys = OrbitalSystem::new(LUNA.0);
    let mut id = ObjectIdTracker::new();

    for _ in 0..5 {
        subsys.add_object(
            id.next(),
            Orbit {
                eccentricity: rand(0.2, 0.5),
                semi_major_axis: rand(100.0, 400.0),
                arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                retrograde: rand(0.0, 1.0) < 0.3,
                primary_mass: LUNA.0.mass,
                time_at_periapsis: Nanotime::default(),
            },
        );
    }

    subsys
}

pub fn earth_moon_example_one() -> OrbitalSystem {
    let mut system = OrbitalSystem::new(EARTH);

    let mut id = ObjectIdTracker::new();

    let luna_id = id.next();

    system.add_object(
        id.next(),
        Orbit::circular(EARTH.radius * 1.1, EARTH.mass, Nanotime::default(), false),
    );

    for _ in 0..200 {
        system.add_object(
            id.next(),
            Orbit {
                eccentricity: rand(0.1, 0.8),
                semi_major_axis: rand(50.0, 2600.0),
                arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                retrograde: rand(0.0, 1.0) < 0.3,
                primary_mass: EARTH.mass,
                time_at_periapsis: Nanotime::default(),
            },
        );
    }

    for vel in 400..1000 {
        let r = Vec2::new(2000.0, 0.0);
        let v = rotate(
            Vec2::X * vel as f32 / 10.0,
            std::f32::consts::PI / (if vel < 700 { 1.9 } else { 2.1 }),
        );
        let o = Orbit::from_pv(r, v, EARTH.mass, Nanotime::default());
        system.add_object(id.next(), o);
    }

    for _ in 0..100 {
        system.add_object(
            id.next(),
            Orbit {
                eccentricity: rand(0.1, 0.5),
                semi_major_axis: rand(5000.0, 9000.0),
                arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                retrograde: rand(0.0, 1.0) < 0.3,
                primary_mass: EARTH.mass,
                time_at_periapsis: Nanotime::default(),
            },
        );
    }

    let mut subsys = OrbitalSystem::new(LUNA.0);

    for _ in 0..5 {
        subsys.add_object(
            id.next(),
            Orbit {
                eccentricity: rand(0.2, 0.5),
                semi_major_axis: rand(100.0, 400.0),
                arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                retrograde: rand(0.0, 1.0) < 0.3,
                primary_mass: LUNA.0.mass,
                time_at_periapsis: Nanotime::default(),
            },
        );
    }

    system.add_subsystem(luna_id, LUNA.1, subsys);

    // let asteroid = (
    //     Body::new(10.0, 2.0, 60.0),
    //     Orbit {
    //         eccentricity: 0.2,
    //         semi_major_axis: LUNA.1.semi_major_axis * 2.0,
    //         arg_periapsis: 0.4,
    //         retrograde: false,
    //         primary_mass: EARTH.mass,
    //         time_at_periapsis: Nanotime::default(),
    //     },
    // );

    // let mut ast = OrbitalSystem::new(asteroid.0);

    // ast.add_object(
    //     id.next(),
    //     Orbit::circular(13.0, asteroid.0.mass, Nanotime::default(), false),
    // );

    // system.add_subsystem(id.next(), asteroid.1, ast);

    system
}

pub fn earth_moon_example_two() -> OrbitalSystem {
    let mut system = OrbitalSystem::new(EARTH);

    let mut id = ObjectIdTracker::new();

    system.add_object(
        id.next(),
        Orbit::circular(EARTH.radius * 1.1, EARTH.mass, Nanotime::default(), false),
    );

    for vel in (180..200).step_by(2) {
        let angle = 0.3;
        system.add_object(
            id.next(),
            Orbit::from_pv(
                rotate(Vec2::Y * -600.0, angle),
                rotate(Vec2::X * vel as f32, angle),
                EARTH.mass,
                Nanotime::default(),
            ),
        );
    }

    let subsys = OrbitalSystem::new(LUNA.0);
    system.add_subsystem(id.next(), LUNA.1, subsys);

    system
}

pub fn sun_jupiter_lagrange() -> OrbitalSystem {
    let sun = Body {
        mass: 1000.0,
        radius: 100.0,
        soi: 100000.0,
    };

    let mut system: OrbitalSystem = OrbitalSystem::new(sun);

    let mut id = ObjectIdTracker::new();

    let jupiter = Body {
        mass: sun.mass * 0.000954588,
        radius: 20.0,
        soi: 500.0,
    };

    let jupiter_orbit = Orbit {
        eccentricity: 0.0,
        arg_periapsis: 0.0,
        semi_major_axis: 5000.0,
        retrograde: false,
        primary_mass: sun.mass,
        time_at_periapsis: Nanotime::default(),
    };

    system.add_subsystem(id.next(), jupiter_orbit, OrbitalSystem::new(jupiter));

    // let s = system.add_object(Vec2::ZERO, Some(sun));

    // system.add_object(KeplerPropagator::new(jupiter_orbit, s), Some(jupiter));

    for _ in 0..600 {
        let orbit = Orbit {
            eccentricity: rand(0.0, 0.3),
            semi_major_axis: rand(4000.0, 6000.0),
            arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
            retrograde: false,
            primary_mass: sun.mass,
            time_at_periapsis: Nanotime::default(),
        };
        system.add_object(id.next(), orbit);
    }

    system
}

pub fn consistency_example() -> OrbitalSystem {
    let mut system: OrbitalSystem = OrbitalSystem::new(EARTH);
    let mut ids = ObjectIdTracker::new();

    let mut orbits = vec![];

    let r = 1000.0 * Vec2::X;
    let v0 = Vec2::new(0.0, 30.0);
    for angle in [0.6] {
        let pos = rotate(r, angle);
        for vx in (-200..=200).step_by(10) {
            for vy in (-200..=200).step_by(10) {
                let o = Orbit::from_pv(
                    pos,
                    v0 + Vec2::new(vx as f32, vy as f32),
                    EARTH.mass,
                    Nanotime::default(),
                );
                orbits.push(o);
            }
        }
    }

    for orbit in orbits {
        system.add_object(ids.next(), orbit);
    }

    system
}

pub fn single_hyperbolic() -> OrbitalSystem {
    let mut system: OrbitalSystem = OrbitalSystem::new(EARTH);
    let orbit = Orbit::from_pv((400.0, 0.0), (0.0, 260.0), EARTH.mass, Nanotime(0));
    system.add_object(ObjectId(0), orbit);
    system
}

pub fn default_example() -> OrbitalSystem {
    consistency_example()
}
