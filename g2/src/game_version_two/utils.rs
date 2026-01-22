use bevy::prelude::*;
use game::starling::parts::{PartCoord, Rotation};

pub fn rotate_ccw(p: PartCoord) -> PartCoord {
    IVec2::Y.rotate(p.inner()).into()
}

/// Given the coordinate of a part in the grid, the parts rotation,
/// and a sample point on the grid, returns sample point expressed
/// in the part-fixed frame.
///
/// g: grid frame origin
/// p: part frame origin
/// o: sample point
/// gp_grid: the vector from g to p, expressed in the grid frame
/// part_rot: rotation between grid and part frame
/// go_grid: the vector from g to o, expressed in the grid frame
///
/// There should be a docs image about this.
pub fn grid_to_part_local(gp_grid: PartCoord, part_rot: Rotation, go_grid: PartCoord) -> PartCoord {
    let po_grid = go_grid - gp_grid;

    let po_part = match part_rot {
        Rotation::East => po_grid,
        Rotation::North => rotate_ccw(rotate_ccw(rotate_ccw(po_grid))),
        Rotation::West => rotate_ccw(rotate_ccw(po_grid)),
        Rotation::South => rotate_ccw(po_grid),
    };

    po_part
}

#[test]
fn grid_to_part_local_test() {
    assert_eq!(
        grid_to_part_local((5, 6).into(), Rotation::East, (10, 3).into()),
        PartCoord::new((5, -3).into())
    );
    assert_eq!(
        grid_to_part_local((5, 6).into(), Rotation::North, (7, 12).into()),
        PartCoord::new((6, -2).into())
    );
    assert_eq!(
        grid_to_part_local((6, 4).into(), Rotation::West, (3, 8).into()),
        PartCoord::new((3, -4).into())
    );
    assert_eq!(
        grid_to_part_local((6, 4).into(), Rotation::South, (12, 2).into()),
        PartCoord::new((2, 6).into())
    );
}

pub fn swallow_optional(In(_): In<Option<()>>) {}

pub fn swallow_result(In(_): In<Result>) {}
