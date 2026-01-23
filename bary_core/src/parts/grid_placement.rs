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

    pub fn origin(&self) -> PartCoord {
        match self.rotation {
            Rotation::East => self.bottom_left,
            Rotation::North => self.bottom_right(),
            Rotation::West => self.top_right(),
            Rotation::South => self.top_left(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_placement_math_east() {
        //
        //        ^
        //        |   EAST
        // (1, 4) |          (5, 4)
        //        *--*--*--*--*
        //        |  |  |  |  |
        //        *--*--*--*--*
        //        |  |  |  |  |
        //        *--*--*--*--*
        //        |  |  |  |  |
        //        *--*--*--*--*------->
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
    }

    #[test]
    fn grid_placement_math_north() {
        //
        //              ^
        //     NORTH    |
        // (5, 6) *--*--* (7, 6)
        //        |  |  |
        //        *--*--*
        //        |  |  |
        //        *--*--*
        //        |  |  |
        //    <---*--*--* (7, 3)
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
    }
}
