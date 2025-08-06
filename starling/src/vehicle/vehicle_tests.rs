#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::prelude::*;

    #[test]
    fn trivial_vehicle() {
        let generic = Generic::new(
            "".to_string(),
            UVec2::new(10, 10),
            PartLayer::Structural,
            Mass::kilograms(400),
        );
        let part = PartPrototype::Generic(generic);

        let vehicle = Vehicle::from_parts(
            "".to_string(),
            "".to_string(),
            vec![(IVec2::ZERO, Rotation::East, part)],
            HashSet::new(),
        );

        assert_eq!(vehicle.total_mass(), Mass::kilograms(400));
        assert_eq!(vehicle.moment_of_inertia(), 0.0);
        assert_eq!(vehicle.center_of_mass(), DVec2::splat(0.25));

        let aabb = vehicle.aabb();
        assert_eq!(aabb.span, Vec2::splat(0.5));
        assert_eq!(aabb.center, Vec2::splat(0.25));
    }

    #[test]
    fn vehicle_with_thruster() {
        let generic = Generic::new(
            "".to_string(),
            UVec2::new(10, 10),
            PartLayer::Structural,
            Mass::kilograms(400),
        );
        let frame = PartPrototype::Generic(generic);

        let thruster = ThrusterModel::main_thruster(5000.0, 3500.0);
        let thruster = PartPrototype::Thruster(thruster);

        let vehicle = Vehicle::from_parts(
            "".into(),
            "".into(),
            vec![
                (IVec2::ZERO, Rotation::East, frame),
                (IVec2::splat(10), Rotation::East, thruster),
            ],
            HashSet::new(),
        );

        assert_eq!(vehicle.total_mass(), Mass::kilograms(1200));
        assert_eq!(vehicle.moment_of_inertia(), 0.0);
        assert_eq!(vehicle.center_of_mass(), DVec2::splat(0.25));

        let aabb = vehicle.aabb();
        assert_eq!(aabb.span, Vec2::splat(0.5));
        assert_eq!(aabb.center, Vec2::splat(0.25));
    }
}
