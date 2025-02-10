use crate::core::*;
use crate::orbit::*;
use bevy::math::Vec2;

pub fn make_earth() -> Body {
    Body::new(63.0, 1000.0, 15000.0)
}

pub fn make_luna() -> (Body, Orbit) {
    (
        Body::new(22.0, 10.0, 800.0),
        Orbit::circular(3800.0, make_earth(), Nanotime(-40 * 1000000000), false),
    )
}

pub fn just_the_moon() -> (OrbitalTree, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();

    let moon_id = id.next();

    let luna = PlanetarySystem::new(moon_id, "Luna", make_luna().0);
    let mut tree = OrbitalTree::new(&luna);

    for _ in 0..5 {
        tree.add_object(
            id.next(),
            moon_id,
            Orbit::circular(rand(200.0, 400.0), luna.body, Nanotime(0), false),
            Nanotime(0),
        );
    }

    (tree, id)
}

pub fn earth_moon_example_one() -> (OrbitalTree, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();

    let mut earth = PlanetarySystem::new(id.next(), "Earth", make_earth());
    let luna = PlanetarySystem::new(id.next(), "Luna", make_luna().0);
    let ast = PlanetarySystem::new(id.next(), "Asteroid", Body::new(10.0, 2.0, 60.0));

    earth.orbit(make_luna().1, luna.clone());
    earth.orbit(
        Orbit::circular(
            make_luna().1.semi_major_axis * 2.0,
            earth.body,
            Nanotime(0),
            false,
        ),
        ast.clone(),
    );

    let mut tree = OrbitalTree::new(&earth);

    tree.add_object(
        id.next(),
        earth.id,
        Orbit::circular(earth.body.radius * 1.1, earth.body, Nanotime(0), false),
        Nanotime(0),
    );

    for _ in 0..200 {
        let r = randvec(700.0, 2400.0);
        let v = randvec(45.0, 70.0);
        let o = Orbit::from_pv((r, v), earth.body, Nanotime(0));
        if let Some(o) = o {
            tree.add_object(id.next(), earth.id, o, Nanotime(0));
        }
    }

    for _ in 0..100 {
        let r = randvec(5000.0, 9000.0);
        let v = randvec(30.0, 70.0);
        let o = Orbit::from_pv((r, v), earth.body, Nanotime(0));
        if let Some(o) = o {
            tree.add_object(id.next(), earth.id, o, Nanotime(0));
        }
    }

    for _ in 0..5 {
        tree.add_object(
            id.next(),
            luna.id,
            Orbit::circular(rand(200.0, 400.0), luna.body, Nanotime(0), false),
            Nanotime(0),
        );
    }

    tree.add_object(
        id.next(),
        ast.id,
        Orbit::circular(13.0, ast.body, Nanotime(0), false),
        Nanotime(0),
    );

    (tree, id)
}

pub fn earth_moon_example_two() -> (OrbitalTree, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();
    let mut earth = PlanetarySystem::new(id.next(), "Earth", make_earth());
    let luna = PlanetarySystem::new(id.next(), "Luna", make_luna().0);

    earth.orbit(make_luna().1, luna);

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
                make_earth(),
                Nanotime(0),
            )
            .unwrap(),
            Nanotime(0),
        );
    }

    (tree, id)
}

pub fn sun_jupiter_lagrange() -> (OrbitalTree, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();

    let mut sun: PlanetarySystem = PlanetarySystem::new(
        id.next(),
        "Sol",
        Body {
            mass: 1000.0,
            radius: 100.0,
            soi: 100000.0,
        },
    );

    let jupiter = Body {
        mass: sun.body.mass * 0.000954588,
        radius: 20.0,
        soi: 500.0,
    };

    let jupiter_orbit = Orbit::circular(5000.0, sun.body, Nanotime(0), false);

    sun.orbit(
        jupiter_orbit,
        PlanetarySystem::new(id.next(), "Jupiter", jupiter),
    );

    let mut tree = OrbitalTree::new(&sun);

    for _ in 0..600 {
        let radius = rand(4000.0, 6000.0);
        let orbit = Orbit::circular(radius, sun.body, Nanotime(0), false);
        tree.add_object(id.next(), sun.id, orbit, Nanotime(0));
    }

    (tree, id)
}

pub fn consistency_example() -> (OrbitalTree, ObjectIdTracker) {
    let mut id = ObjectIdTracker::new();

    let earth = PlanetarySystem::new(id.next(), "Earth", make_earth());

    let mut orbits = vec![];

    let r = 1000.0 * Vec2::X;
    let v0 = Vec2::new(0.0, 30.0);
    for angle in [0.6] {
        let pos = rotate(r, angle);
        for vx in (-200..=200).step_by(10) {
            for vy in (-200..=200).step_by(10) {
                let o = Orbit::from_pv(
                    (pos, v0 + Vec2::new(vx as f32, vy as f32)),
                    make_earth(),
                    Nanotime(0),
                );
                if let Some(o) = o {
                    orbits.push(o);
                }
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
    let earth: PlanetarySystem = PlanetarySystem::new(id.next(), "Earth", make_earth());
    let mut tree = OrbitalTree::new(&earth);
    let orbit = Orbit::from_pv(((400.0, 0.0), (0.0, 260.0)), make_earth(), Nanotime(0)).unwrap();
    tree.add_object(id.next(), earth.id, orbit, Nanotime(0));
    (tree, id)
}

pub fn default_example() -> (OrbitalTree, ObjectIdTracker) {
    earth_moon_example_one()
}
