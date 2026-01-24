use bevy::transform::components::Transform;

use crate::math::*;

#[derive(Clone, Copy, Debug)]
pub struct GridPlacement {
    /// the coordinate of the bottom
    /// left corner of the part
    bottom_left: PartCoord,

    /// the rotation of the part
    /// with respect to the root grid
    rotation: Rotation,

    /// the dimension of the part itself,
    /// irrespective of any rotation
    part_local_dims: UVec2,
}

impl GridPlacement {
    pub fn new(bottom_left: impl Into<PartCoord>, rot: Rotation, dims: impl Into<UVec2>) -> Self {
        Self {
            bottom_left: bottom_left.into(),
            rotation: rot,
            part_local_dims: dims.into(),
        }
    }

    pub fn part_aligned_dims(&self) -> PartCoord {
        self.part_local_dims.as_ivec2().into()
    }

    pub fn grid_aligned_dims(&self) -> PartCoord {
        match self.rotation {
            Rotation::East | Rotation::West => {
                UVec2::new(self.part_local_dims.x, self.part_local_dims.y).into()
            }
            Rotation::North | Rotation::South => {
                UVec2::new(self.part_local_dims.y, self.part_local_dims.x).into()
            }
        }
    }

    pub fn set_bottom_left(&mut self, p: PartCoord) {
        self.bottom_left = p;
    }

    pub fn bottom_left(&self) -> PartCoord {
        self.bottom_left
    }

    pub fn bottom_right(&self) -> PartCoord {
        let off: PartCoord = self.grid_aligned_dims().0.with_y(0).into();
        self.bottom_left + off
    }

    pub fn top_left(&self) -> PartCoord {
        let off: PartCoord = self.grid_aligned_dims().0.with_x(0).into();
        self.bottom_left + off
    }

    pub fn top_right(&self) -> PartCoord {
        let off = self.grid_aligned_dims();
        self.bottom_left + off
    }

    pub fn rot(&self) -> Rotation {
        self.rotation
    }

    pub fn set_rot(&mut self, rot: Rotation) {
        self.rotation = rot;
    }

    pub fn origin(&self) -> PartCoord {
        match self.rotation {
            Rotation::East => self.bottom_left,
            Rotation::North => self.bottom_right(),
            Rotation::West => self.top_right(),
            Rotation::South => self.top_left(),
        }
    }

    pub fn origin_isometry(&self) -> Isometry2d {
        let rot = self.rotation.to_angle() as f32;
        Isometry2d::new(self.origin().to_meters(), rot.into())
    }

    pub fn center_isometry(&self) -> Isometry2d {
        let rot = self.rotation.to_angle() as f32;
        let half_dims = self.part_aligned_dims().to_meters() / 2.0;
        let offset = rotate(half_dims, rot);
        Isometry2d::new(self.origin().to_meters() + offset, rot.into())
    }

    pub fn cells(&self) -> impl Iterator<Item = PartCoord> + use<'_> {
        let wh = self.grid_aligned_dims().inner();
        let iter_y = 0..wh.y;
        let iter_coords = iter_y.flat_map(move |y| {
            let iter_x = 0..wh.x;
            iter_x.map(move |x| (x, y))
        });
        iter_coords.map(|p| self.bottom_left + PartCoord::new(p))
    }
}

pub fn isometry_to_transform(iso: Isometry2d) -> Transform {
    let iso = Isometry3d::new(
        iso.translation.extend(0.0),
        Quat::from_rotation_z(iso.rotation.as_radians()),
    );
    Transform::from_isometry(iso)
}

impl std::ops::Add<PartCoord> for GridPlacement {
    type Output = GridPlacement;
    fn add(self, rhs: PartCoord) -> Self::Output {
        let mut s = self;
        s.bottom_left += rhs;
        s
    }
}

impl std::ops::AddAssign<PartCoord> for GridPlacement {
    fn add_assign(&mut self, rhs: PartCoord) {
        self.bottom_left += rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_placement_math_east() {
        //        y
        //        ^
        //        |   EAST
        // (1, 4) |          (5, 4)
        //        *--*--*--*--*
        //        |  |  |  |  |
        //        *--*--*--*--*
        //        |  |  |  |  |
        //        *--*--*--*--*
        //        |  |  |  |  |
        //        *--*--*--*--*-------> x
        //      (1, 1)       (5, 1)
        //
        let gp = GridPlacement::new((1, 1), Rotation::East, (4, 3));

        assert_eq!(gp.part_aligned_dims(), (4, 3).into());
        assert_eq!(gp.grid_aligned_dims(), (4, 3).into());

        assert_eq!(gp.bottom_left(), (1, 1).into());
        assert_eq!(gp.bottom_right(), (5, 1).into());
        assert_eq!(gp.top_left(), (1, 4).into());
        assert_eq!(gp.top_right(), (5, 4).into());

        assert_eq!(gp.origin(), (1, 1).into());

        assert_eq!(
            gp.origin_isometry(),
            Isometry2d::new((0.25, 0.25).into(), Rot2::IDENTITY)
        );

        assert_eq!(
            gp.center_isometry(),
            Isometry2d::new((0.75, 0.625).into(), Rot2::IDENTITY)
        );
    }

    #[test]
    fn grid_placement_math_north() {
        //              x
        //              ^
        //     NORTH    |
        // (5, 6) *--*--* (7, 6)
        //        |  |  |
        //        *--*--*
        //        |  |  |
        //        *--*--*
        //        |  |  |
        //  y <---*--*--* (7, 3)
        //     (5, 3)
        //
        let gp = GridPlacement::new((5, 3), Rotation::North, (3, 2));

        assert_eq!(gp.part_aligned_dims(), (3, 2).into());
        assert_eq!(gp.grid_aligned_dims(), (2, 3).into());

        assert_eq!(gp.bottom_left(), (5, 3).into());
        assert_eq!(gp.bottom_right(), (7, 3).into());
        assert_eq!(gp.top_left(), (5, 6).into());
        assert_eq!(gp.top_right(), (7, 6).into());

        assert_eq!(gp.origin(), (7, 3).into());

        assert_eq!(
            gp.origin_isometry(),
            Isometry2d::new((1.75, 0.75).into(), (PI / 2.0).into())
        );

        assert_eq!(
            gp.center_isometry(),
            Isometry2d::new((1.5, 1.125).into(), (PI / 2.0).into())
        );
    }

    #[test]
    fn grid_placement_math_west() {
        //
        //   x <---*--*--*--* (15, 9)
        // (12, 9) |  |  |  |
        //         *--*--*--*
        //   WEST  |  |  |  |
        //         *--*--*--*
        //         |  |  |  |
        //         *--*--*--*
        //         |  |  |  |
        //         *--*--*--*
        //      (12, 5)     | (15, 5)
        //                  |
        //                  v
        //                  y
        let gp = GridPlacement::new((12, 5), Rotation::West, (3, 4));

        assert_eq!(gp.part_aligned_dims(), (3, 4).into());
        assert_eq!(gp.grid_aligned_dims(), (3, 4).into());

        assert_eq!(gp.bottom_left(), (12, 5).into());
        assert_eq!(gp.bottom_right(), (15, 5).into());
        assert_eq!(gp.top_left(), (12, 9).into());
        assert_eq!(gp.top_right(), (15, 9).into());

        assert_eq!(gp.origin(), (15, 9).into());

        assert_eq!(
            gp.origin_isometry(),
            Isometry2d::new((3.75, 2.25).into(), PI.into())
        );

        assert_eq!(
            gp.center_isometry(),
            Isometry2d::new((3.375, 1.75).into(), PI.into())
        );
    }

    #[test]
    fn grid_placement_math_south() {
        //
        //         *--*--*--*--*----> y
        // (2, 17) |  |  |  |  | (12, 17)
        //         *--*--*--*--*
        //   SOUTH |  |  |  |  |
        //         *--*--*--*--*
        //         |  |  |  |  |
        //       -_-*-_-*-_-*-_-*-
        //         |  |  |  |  |
        //         *--*--*--*--*
        //         |  |  |  |  |
        // (2, -8) *--*--*--*--* (12, -8)
        //         |
        //         |
        //         v
        //         x

        let bottom_left = (2, -8);
        let rot = Rotation::South;
        let dims = (25, 10);

        let gp = GridPlacement::new(bottom_left, rot, dims);

        assert_eq!(gp.part_aligned_dims(), (25, 10).into());
        assert_eq!(gp.grid_aligned_dims(), (10, 25).into());

        assert_eq!(gp.bottom_left(), (2, -8).into());
        assert_eq!(gp.bottom_right(), (12, -8).into());
        assert_eq!(gp.top_left(), (2, 17).into());
        assert_eq!(gp.top_right(), (12, 17).into());

        assert_eq!(gp.origin(), (2, 17).into());

        assert_eq!(
            gp.origin_isometry(),
            Isometry2d::new((0.5, 4.25).into(), (1.5 * PI).into())
        );

        assert_eq!(
            gp.center_isometry(),
            Isometry2d::new((1.75, 1.125).into(), (1.5 * PI).into())
        );
    }

    #[test]
    fn grid_placement_cells_iterator() {
        //         x ^
        //           |
        //      @@@@@@
        //      @@@@@@
        // y <--@@@@@@
        //      \
        //     (3, 2)

        let bottom_left = (3, 2);
        let rot = Rotation::North;
        let dims = (3, 6);

        let gp = GridPlacement::new(bottom_left, rot, dims);

        assert_eq!(gp.grid_aligned_dims(), (6, 3).into());

        assert_eq!(gp.bottom_left(), (3, 2).into());
        assert_eq!(gp.top_right(), (9, 5).into());

        let mut iter = gp.cells();

        assert_eq!(iter.next(), Some((3, 2).into()));
        assert_eq!(iter.next(), Some((4, 2).into()));
        assert_eq!(iter.next(), Some((5, 2).into()));
        assert_eq!(iter.next(), Some((6, 2).into()));
        assert_eq!(iter.next(), Some((7, 2).into()));
        assert_eq!(iter.next(), Some((8, 2).into()));

        assert_eq!(iter.next(), Some((3, 3).into()));
        assert_eq!(iter.next(), Some((4, 3).into()));
        assert_eq!(iter.next(), Some((5, 3).into()));
        assert_eq!(iter.next(), Some((6, 3).into()));
        assert_eq!(iter.next(), Some((7, 3).into()));
        assert_eq!(iter.next(), Some((8, 3).into()));

        assert_eq!(iter.next(), Some((3, 4).into()));
        assert_eq!(iter.next(), Some((4, 4).into()));
        assert_eq!(iter.next(), Some((5, 4).into()));
        assert_eq!(iter.next(), Some((6, 4).into()));
        assert_eq!(iter.next(), Some((7, 4).into()));
        assert_eq!(iter.next(), Some((8, 4).into()));

        assert_eq!(iter.next(), None);
    }
}
