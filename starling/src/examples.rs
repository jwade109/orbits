use crate::math::*;
use crate::nanotime::Nanotime;
use crate::orbits::{Body, SparseOrbit};
use crate::quantities::*;
use crate::scenario::{ObjectIdTracker, PlanetarySystem};

pub fn make_earth() -> Body {
    Body::with_mass(63.0, 1000.0, 15000.0)
}

pub fn make_earth_inf_soi() -> Body {
    Body::with_mass(63.0, 1000.0, 10000000.0)
}

pub fn make_luna() -> (Body, SparseOrbit) {
    (
        Body::with_mass(22.0, 10.0, 800.0),
        SparseOrbit::circular(3800.0, make_earth(), Nanotime::secs(-40), false),
    )
}

pub fn rss() -> PlanetarySystem {
    let mut id = ObjectIdTracker::new();
    let earth_body = Body::with_mu(EARTH_RADIUS, EARTH_MU, EARTH_SOI);
    let mut earth = PlanetarySystem::new(id.next(), "Earth", earth_body);

    let luna_body = Body::with_mu(LUNA_RADIUS, LUNA_MU, LUNA_SOI);
    let mut luna = PlanetarySystem::new(id.next(), "Luna", luna_body);
    let luna_orbit = SparseOrbit::circular(
        LUNA_ORBITAL_RADIUS as f64,
        earth_body,
        Nanotime::zero(),
        false,
    );

    let ast_body = Body::with_mu(LUNA_RADIUS * 0.01, LUNA_MU * 0.0001, LUNA_RADIUS * 0.02);
    let ast = PlanetarySystem::new(id.next(), "Asteroid", ast_body);
    let ast_orbit = SparseOrbit::circular(
        LUNA_RADIUS * 1.5 as f64,
        luna_body,
        Nanotime::zero(),
        true,
    );

    luna.orbit(ast_orbit, ast);
    earth.orbit(luna_orbit, luna);

    earth
}

pub fn default_example() -> PlanetarySystem {
    rss()
}
