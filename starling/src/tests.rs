#![allow(unused)]

use crate::core::*;
use crate::examples::*;
use crate::orbit::*;
use approx::assert_relative_eq;
use bevy::math::Vec2;

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

    assert_eq!(o1.period().unwrap(), o2.period().unwrap());

    let o1_f = Anomaly::with_ecc(o1.eccentricity, -3.083711);

    assert_relative_eq!(o1.ta_at_time(t).as_f32(), o1_f.as_f32(), epsilon = 0.01);
    assert_relative_eq!(o2.ta_at_time(t).as_f32(), o1_f.as_f32(), epsilon = 0.01);

    let z = o1.period().unwrap();

    for i in -5..5 {
        let t = o1.period().unwrap() * i;
        assert_relative_eq!(
            o1.pv_at_time(t).pos.x,
            o2.pv_at_time(t).pos.x,
            epsilon = 0.5
        );
        assert_relative_eq!(
            o1.pv_at_time(t).pos.y,
            o2.pv_at_time(t).pos.y,
            epsilon = 0.5
        );
    }
}

pub fn test_scenario_one() -> OrbitalSystem {
    let mut system = OrbitalSystem::new(ObjectId(0), EARTH);

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
