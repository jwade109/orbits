use starling::prelude::*;

fn main() {
    let mut vehicle = Vehicle::new();

    let proto = PartPrototype::Cargo(Cargo::new(
        "cargo".into(),
        Mass::kilograms(200),
        Mass::kilograms(3000),
        UVec2::new(30, 20),
    ));

    vehicle.add_part(proto, IVec2::ZERO, Rotation::East);

    dbg!(vehicle);
}
