use crate::core::*;
use crate::orbit::*;
use bevy::math::Vec2;

#[cfg(test)]
use approx::assert_relative_eq;

pub const EARTH: Body = Body::new(63.0, 1000.0, 15000.0);

pub const LUNA: (Body, Orbit) = (
    Body::new(22.0, 10.0, 800.0),
    Orbit::circular(3800.0, 0.0, EARTH.mass),
);

pub fn earth_moon_example_one() -> OrbitalSystem {
    let mut system = OrbitalSystem::new(EARTH);

    for _ in 0..200 {
        system.add_object(Orbit {
            eccentricity: rand(0.1, 0.8),
            semi_major_axis: rand(50.0, 2600.0),
            arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
            retrograde: rand(0.0, 1.0) < 0.3,
            primary_mass: EARTH.mass,
        });
    }

    for _ in 0..100 {
        system.add_object(Orbit {
            eccentricity: rand(0.1, 0.5),
            semi_major_axis: rand(5000.0, 9000.0),
            arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
            retrograde: rand(0.0, 1.0) < 0.3,
            primary_mass: EARTH.mass,
        });
    }

    let mut subsys = OrbitalSystem::new(LUNA.0);

    for _ in 0..5 {
        subsys.add_object(Orbit {
            eccentricity: rand(0.2, 0.5),
            semi_major_axis: rand(100.0, 400.0),
            arg_periapsis: rand(0.0, std::f32::consts::PI * 2.0),
            retrograde: rand(0.0, 1.0) < 0.3,
            primary_mass: LUNA.0.mass,
        });
    }

    system.subsystems.push((LUNA.1, subsys));

    system
}

pub fn sun_jupiter_lagrange() -> OrbitalSystem {
    let mut system = OrbitalSystem::new(Body {
        mass: 1000.0,
        radius: 100.0,
        soi: 100000.0,
    });

    // let jupiter = Body {
    //     mass: sun.mass * 0.000954588,
    //     radius: 20.0,
    //     soi: 500.0,
    // };

    // let jupiter_orbit = Orbit {
    //     eccentricity: 0.0,
    //     arg_periapsis: 0.0,
    //     semi_major_axis: 5000.0,
    //     retrograde: false,
    //     primary_mass: sun.mass,
    // };

    // let s = system.add_object(Vec2::ZERO, Some(sun));

    // system.add_object(KeplerPropagator::new(jupiter_orbit, s), Some(jupiter));

    // for _ in 0..600 {
    //     let r = rand(4000.0, 6000.0);
    //     let orbit = Orbit::circular(r, 0.0, sun.mass);
    //     system.add_object(KeplerPropagator::new(orbit, s), None);
    // }

    system
}

pub fn patched_conics_scenario() -> OrbitalSystem {
    let mut system = OrbitalSystem::new(EARTH);

    // let e = system.add_object(EARTH.1, Some(EARTH.0));

    // system.add_object(
    //     KeplerPropagator::new(Orbit::circular(5000.0, 0.0, EARTH.0.mass), e),
    //     Some(LUNA.0),
    // );

    // for _ in 0..30 {
    //     let r = randvec(200.0, 201.0);
    //     let v = Vec2::from_angle(std::f32::consts::PI / 2.0)
    //         .rotate(r)
    //         .normalize()
    //         * 340.0;
    //     system.add_object(
    //         KeplerPropagator::new(Orbit::from_pv(r, v, EARTH.0.mass), e),
    //         None,
    //     );
    // }

    system
}

pub fn default_example() -> OrbitalSystem {
    earth_moon_example_one()
}
