#![allow(unused)]

use crate::core::*;
use crate::examples::*;
use approx::assert_relative_eq;
use bevy::math::Vec2;
use crate::orbit::*;

const TEST_BODY: Body = Body {
    mass: 1000.0,
    radius: 50.0,
    soi: f32::MAX,
};

const TEST_POSITION: Vec2 = Vec2::new(500.0, 300.0);
const TEST_VELOCITY: Vec2 = Vec2::new(-200.0, 0.0);

#[test]
fn orbit_construction() {
    let o1 = Orbit::from_pv(TEST_POSITION, TEST_VELOCITY, TEST_BODY.mass, Nanotime(0));
    let o2 = Orbit::from_pv(TEST_POSITION, -TEST_VELOCITY, TEST_BODY.mass, Nanotime(0));

    let true_h = TEST_POSITION.extend(0.0).cross(TEST_VELOCITY.extend(0.0)).z;

    assert_relative_eq!(o1.angular_momentum(), true_h);
    assert!(o1.angular_momentum() > 0.0);
    assert!(!o1.retrograde);

    assert_relative_eq!(o2.angular_momentum(), true_h);
    assert!(o1.angular_momentum() > 0.0);
    assert!(o2.retrograde);

    let t = o1.period().unwrap() * 0.7;

    let o1_f = Anomaly::with_ecc(o1.eccentricity, -3.083711);

    assert_relative_eq!(o1.ta_at_time(t).as_f32(), o1_f.as_f32(), epsilon = 0.01);
    assert_relative_eq!(o2.ta_at_time(t).as_f32(), o1_f.as_f32(), epsilon = 0.01);

    let z = Nanotime(0);

    assert_relative_eq!(o1.pv_at_time(z).pos.x, o2.pv_at_time(z).pos.x, epsilon = 0.01);
    assert_relative_eq!(o1.pv_at_time(z).pos.y, o2.pv_at_time(z).pos.y, epsilon = 0.01);
}

pub fn test_scenario_one() -> OrbitalSystem {
    let mut system = OrbitalSystem::new(EARTH);

    // let rid = system.add_object(Vec2::ZERO, Some(TEST_BODY));

    // system.add_object(
    //     KeplerPropagator::new(
    //         Orbit::from_pv(TEST_POSITION, TEST_VELOCITY, TEST_BODY.mass),
    //         rid,
    //     ),
    //     None,
    // );

    // system.add_object(
    //     KeplerPropagator::new(
    //         Orbit::from_pv(TEST_POSITION, -TEST_VELOCITY, TEST_BODY.mass),
    //         rid,
    //     ),
    //     None,
    // );

    system
}
