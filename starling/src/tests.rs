#![allow(unused)]

use crate::core::*;
use crate::examples::*;
use crate::propagator::*;
use approx::assert_relative_eq;
use bevy::math::Vec2;
use std::time::Duration;

const TEST_BODY: Body = Body {
    mass: 1000.0,
    radius: 50.0,
    soi: f32::MAX,
};

const TEST_POSITION: Vec2 = Vec2::new(500.0, 300.0);
const TEST_VELOCITY: Vec2 = Vec2::new(-200.0, 0.0);

#[test]
fn orbit_construction() {
    let o1 = Orbit::from_pv(TEST_POSITION, TEST_VELOCITY, TEST_BODY.mass);
    let o2 = Orbit::from_pv(TEST_POSITION, -TEST_VELOCITY, TEST_BODY.mass);

    let true_h = TEST_POSITION.extend(0.0).cross(TEST_VELOCITY.extend(0.0)).z;

    assert_relative_eq!(o1.angular_momentum(), true_h);
    assert!(o1.angular_momentum() > 0.0);
    assert!(!o1.retrograde);

    assert_relative_eq!(o2.angular_momentum(), true_h);
    assert!(o1.angular_momentum() > 0.0);
    assert!(o2.retrograde);

    let o1_f = 4.197201;

    assert_eq!(o1.true_anomaly, o1_f);
    assert_eq!(o2.true_anomaly, std::f32::consts::PI * 2.0 - o1_f);

    assert_relative_eq!(o1.pv().pos.x, o2.pv().pos.x, epsilon = 0.01);
    assert_relative_eq!(o1.pv().pos.y, o2.pv().pos.y, epsilon = 0.01);
}

pub fn test_scenario_one() -> OrbitalSystem {
    let mut system = OrbitalSystem::default();

    let rid = system.add_object(Vec2::ZERO, Some(TEST_BODY));

    system.add_object(
        KeplerPropagator::new(
            Orbit::from_pv(TEST_POSITION, TEST_VELOCITY, TEST_BODY.mass),
            rid,
        ),
        None,
    );

    system.add_object(
        KeplerPropagator::new(
            Orbit::from_pv(TEST_POSITION, -TEST_VELOCITY, TEST_BODY.mass),
            rid,
        ),
        None,
    );

    system
}

#[test]
fn propagation_equality() {
    let mut s1 = earth_moon_example_one();
    let mut s2 = s1.clone();

    let mut s1_events = vec![];
    let mut s2_events = vec![];

    for _ in 0..1000 {
        s1_events.extend(s1.step());
    }

    for _ in 0..1000 {
        s2_events.extend(s2.step());
    }

    assert_eq!(s1.epoch, Duration::from_secs(100));
    assert_eq!(s2.epoch, Duration::from_secs(100));

    assert_eq!(s1_events.len(), s2_events.len());
    assert_eq!(s1.objects.len(), s2.objects.len());
}
