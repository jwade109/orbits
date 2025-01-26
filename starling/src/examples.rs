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

pub fn just_the_moon() -> (OrbitalSystem, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();
    let mut system = OrbitalSystem::new(id.next(), LUNA.0);

    for _ in 0..5 {
        system.add_object(
            id.next(),
            Orbit {
                eccentricity: rand(0.2, 0.5),
                semi_major_axis: rand(100.0, 400.0),
                arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                retrograde: rand(0.0, 1.0) < 0.3,
                primary_mass: LUNA.0.mass,
                time_at_periapsis: Nanotime::default(),
            },
            Nanotime(0),
        );
    }

    (system, id)
}

pub fn earth_moon_example_one() -> (OrbitalSystem, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();

    let mut system = OrbitalSystem::new(id.next(), EARTH);

    system.add_object(
        id.next(),
        Orbit::circular(EARTH.radius * 1.1, EARTH.mass, Nanotime::default(), false),
        Nanotime(0),
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
            Nanotime(0),
        );
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
            Nanotime(0),
        );
    }

    let mut subsys = OrbitalSystem::new(id.next(), LUNA.0);

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
            Nanotime(0),
        );
    }

    system.add_subsystem(LUNA.1, Nanotime(0), subsys);

    let asteroid = (
        Body::new(10.0, 2.0, 60.0),
        Orbit {
            eccentricity: 0.2,
            semi_major_axis: LUNA.1.semi_major_axis * 2.0,
            arg_periapsis: 0.4,
            retrograde: false,
            primary_mass: EARTH.mass,
            time_at_periapsis: Nanotime::default(),
        },
    );

    let mut ast = OrbitalSystem::new(id.next(), asteroid.0);

    ast.add_object(
        id.next(),
        Orbit::circular(13.0, asteroid.0.mass, Nanotime::default(), false),
        Nanotime(0),
    );

    system.add_subsystem(asteroid.1, Nanotime(0), ast);

    (system, id)
}

pub fn earth_moon_example_two() -> (OrbitalSystem, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();
    let mut system = OrbitalSystem::new(id.next(), EARTH);

    system.add_object(
        id.next(),
        Orbit::circular(EARTH.radius * 1.1, EARTH.mass, Nanotime::default(), false),
        Nanotime(0),
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
            Nanotime(0),
        );
    }

    let subsys = OrbitalSystem::new(id.next(), LUNA.0);

    system.add_subsystem(LUNA.1, Nanotime(0), subsys);

    (system, id)
}

pub fn sun_jupiter_lagrange() -> (OrbitalSystem, ObjectIdTracker) {
    let sun = Body {
        mass: 1000.0,
        radius: 100.0,
        soi: 100000.0,
    };

    let mut id = ObjectIdTracker::new();

    let mut system: OrbitalSystem = OrbitalSystem::new(id.next(), sun);

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

    system.add_subsystem(
        jupiter_orbit,
        Nanotime(0),
        OrbitalSystem::new(id.next(), jupiter),
    );

    for _ in 0..600 {
        let orbit = Orbit {
            eccentricity: rand(0.0, 0.3),
            semi_major_axis: rand(4000.0, 6000.0),
            arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
            retrograde: false,
            primary_mass: sun.mass,
            time_at_periapsis: Nanotime::default(),
        };
        system.add_object(id.next(), orbit, Nanotime(0));
    }

    (system, id)
}

pub fn consistency_example() -> (OrbitalSystem, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();

    let mut system: OrbitalSystem = OrbitalSystem::new(id.next(), EARTH);

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
        system.add_object(id.next(), orbit, Nanotime(0));
    }

    (system, id)
}

pub fn single_hyperbolic() -> (OrbitalSystem, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();
    let mut system: OrbitalSystem = OrbitalSystem::new(id.next(), EARTH);
    let orbit = Orbit::from_pv((400.0, 0.0), (0.0, 260.0), EARTH.mass, Nanotime(0));
    system.add_object(id.next(), orbit, Nanotime(0));
    (system, id)
}

pub fn default_example() -> (OrbitalSystem, ObjectIdTracker) {
    earth_moon_example_one()
}
