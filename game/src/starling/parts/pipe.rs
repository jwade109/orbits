use super::parts::PartCoord;
use bevy::math::IVec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PipeGeometry {
    pub start: PartCoord,
    pub end: PartCoord,
    pub x_first: bool,
}

pub enum PipeSegments {
    Single(PartCoord, PartCoord),
    Double(PartCoord, PartCoord, PartCoord),
}

impl PipeGeometry {
    pub fn segments(&self) -> PipeSegments {
        if let Some(b) = get_bend_location(self.start, self.end, self.x_first) {
            PipeSegments::Double(self.start, b, self.end)
        } else {
            PipeSegments::Single(self.start, self.end)
        }
    }
}

/// computes where the bend in a pipe is, based on its starting and
/// ending location. pipes that begin and end on the same x- or y-value
/// do not bend, so this will return None.
pub fn get_bend_location(
    from: impl Into<PartCoord>,
    to: impl Into<PartCoord>,
    x_first: bool,
) -> Option<PartCoord> {
    let from = from.into().inner();
    let to = to.into().inner();

    if from.x == to.x || from.y == to.y {
        return None;
    }

    // x_first determines which axis will be traversed first.
    // x_first = true:
    //     (0, 0) to (1, 1) should pass through (1, 0)
    // x_first = false:
    //     (0, 0) to (1, 1) should pass through (0, 1)

    Some(if x_first {
        PartCoord::new(IVec2::new(to.x, from.y))
    } else {
        PartCoord::new(IVec2::new(from.x, to.y))
    })
}

#[test]
fn pipe_path_computation() {
    assert_eq!(
        get_bend_location((0, 0), (1, 1), true),
        Some(PartCoord::new(IVec2::new(1, 0)))
    );

    assert_eq!(
        get_bend_location((0, 0), (1, 1), false),
        Some(PartCoord::new(IVec2::new(0, 1)))
    );

    assert_eq!(
        get_bend_location((3, 2), (-4, 10), true),
        Some(PartCoord::new(IVec2::new(-4, 2)))
    );

    assert_eq!(
        get_bend_location((3, 2), (-4, 10), false),
        Some(PartCoord::new(IVec2::new(3, 10)))
    );
}
