use crate::core::*;
use crate::orbit::*;
use bevy::math::Vec2;

pub const EARTH: Body = Body::new(63.0, 1000.0, 15000.0);

pub const LUNA: (Body, Orbit) = (
    Body::new(22.0, 10.0, 800.0),
    Orbit::circular(3800.0, EARTH.mass, Nanotime(-40 * 1000000000), false),
);

pub fn just_the_moon() -> (OrbitalTree, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();

    let moon_id = id.next();

    let mut tree = OrbitalTree::new(&Planet::new(moon_id, "Luna", LUNA.0));

    for _ in 0..5 {
        tree.add_object(
            id.next(),
            moon_id,
            Orbit {
                eccentricity: rand(0.2, 0.5),
                semi_major_axis: rand(100.0, 400.0),
                arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                retrograde: rand(0.0, 1.0) < 0.3,
                primary_mass: LUNA.0.mass,
                time_at_periapsis: Nanotime(0),
            },
            Nanotime(0),
        );
    }

    (tree, id)
}

pub fn earth_moon_example_one() -> (OrbitalTree, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();

    let mut earth = Planet::new(id.next(), "Earth", EARTH);
    let luna = Planet::new(id.next(), "Luna", LUNA.0);
    let ast = Planet::new(id.next(), "Asteroid", Body::new(10.0, 2.0, 60.0));

    earth.orbit(LUNA.1, luna.clone());
    earth.orbit(
        Orbit {
            eccentricity: 0.2,
            semi_major_axis: LUNA.1.semi_major_axis * 2.0,
            arg_periapsis: 0.4,
            retrograde: false,
            primary_mass: earth.primary.mass,
            time_at_periapsis: Nanotime(0),
        },
        ast.clone(),
    );

    let mut tree = OrbitalTree::new(&earth);

    tree.add_object(
        id.next(),
        earth.id,
        Orbit::circular(
            earth.primary.radius * 1.1,
            earth.primary.mass,
            Nanotime(0),
            false,
        ),
        Nanotime(0),
    );

    for _ in 0..200 {
        tree.add_object(
            id.next(),
            earth.id,
            Orbit {
                eccentricity: rand(0.1, 0.8),
                semi_major_axis: rand(50.0, 2600.0),
                arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                retrograde: rand(0.0, 1.0) < 0.3,
                primary_mass: earth.primary.mass,
                time_at_periapsis: Nanotime(0),
            },
            Nanotime(0),
        );
    }

    for _ in 0..100 {
        tree.add_object(
            id.next(),
            earth.id,
            Orbit {
                eccentricity: rand(0.1, 0.5),
                semi_major_axis: rand(5000.0, 9000.0),
                arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                retrograde: rand(0.0, 1.0) < 0.3,
                primary_mass: earth.primary.mass,
                time_at_periapsis: Nanotime(0),
            },
            Nanotime(0),
        );
    }

    for _ in 0..5 {
        tree.add_object(
            id.next(),
            luna.id,
            Orbit {
                eccentricity: rand(0.2, 0.5),
                semi_major_axis: rand(100.0, 400.0),
                arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
                retrograde: rand(0.0, 1.0) < 0.3,
                primary_mass: luna.primary.mass,
                time_at_periapsis: Nanotime(0),
            },
            Nanotime(0),
        );
    }

    tree.add_object(
        id.next(),
        ast.id,
        Orbit::circular(13.0, ast.primary.mass, Nanotime(0), false),
        Nanotime(0),
    );

    (tree, id)
}

pub fn earth_moon_example_two() -> (OrbitalTree, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();
    let mut earth = Planet::new(id.next(), "Earth", EARTH);
    let luna = Planet::new(id.next(), "Luna", LUNA.0);

    earth.orbit(LUNA.1, luna);

    let mut tree = OrbitalTree::new(&earth);

    for vel in (180..200).step_by(2) {
        let angle = 0.3;
        tree.add_object(
            id.next(),
            earth.id,
            Orbit::from_pv(
                (
                    rotate(Vec2::Y * -600.0, angle),
                    rotate(Vec2::X * vel as f32, angle),
                ),
                EARTH.mass,
                Nanotime(0),
            ),
            Nanotime(0),
        );
    }

    (tree, id)
}

pub fn sun_jupiter_lagrange() -> (OrbitalTree, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();

    let mut sun: Planet = Planet::new(
        id.next(),
        "Sol",
        Body {
            mass: 1000.0,
            radius: 100.0,
            soi: 100000.0,
        },
    );

    let jupiter = Body {
        mass: sun.primary.mass * 0.000954588,
        radius: 20.0,
        soi: 500.0,
    };

    let jupiter_orbit = Orbit {
        eccentricity: 0.0,
        arg_periapsis: 0.0,
        semi_major_axis: 5000.0,
        retrograde: false,
        primary_mass: sun.primary.mass,
        time_at_periapsis: Nanotime(0),
    };

    sun.orbit(jupiter_orbit, Planet::new(id.next(), "Jupiter", jupiter));

    let mut tree = OrbitalTree::new(&sun);

    for _ in 0..600 {
        let orbit = Orbit {
            eccentricity: rand(0.0, 0.3),
            semi_major_axis: rand(4000.0, 6000.0),
            arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
            retrograde: false,
            primary_mass: sun.primary.mass,
            time_at_periapsis: Nanotime(0),
        };
        tree.add_object(id.next(), sun.id, orbit, Nanotime(0));
    }

    (tree, id)
}

pub fn consistency_example() -> (OrbitalTree, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();

    let earth = Planet::new(id.next(), "Earth", EARTH);

    let mut orbits = vec![];

    let r = 1000.0 * Vec2::X;
    let v0 = Vec2::new(0.0, 30.0);
    for angle in [0.6] {
        let pos = rotate(r, angle);
        for vx in (-200..=200).step_by(10) {
            for vy in (-200..=200).step_by(10) {
                let o = Orbit::from_pv(
                    (pos, v0 + Vec2::new(vx as f32, vy as f32)),
                    EARTH.mass,
                    Nanotime(0),
                );
                orbits.push(o);
            }
        }
    }

    let mut tree = OrbitalTree::new(&earth);

    for orbit in orbits {
        tree.add_object(id.next(), earth.id, orbit, Nanotime(0));
    }

    (tree, id)
}

pub fn single_hyperbolic() -> (OrbitalTree, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();
    let earth: Planet = Planet::new(id.next(), "Earth", EARTH);
    let mut tree = OrbitalTree::new(&earth);
    let orbit = Orbit::from_pv(((400.0, 0.0), (0.0, 260.0)), EARTH.mass, Nanotime(0));
    tree.add_object(id.next(), earth.id, orbit, Nanotime(0));
    (tree, id)
}

pub fn default_example() -> (OrbitalTree, ObjectIdTracker) {
    earth_moon_example_one()
}
