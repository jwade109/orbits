use bary_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipeGeometry {
    pub start: PartCoord,
    pub end: PartCoord,
    pub x_first: bool,
}

pub enum PipeSegments {
    Single(PartCoord, PartCoord),
    Double(PartCoord, PartCoord, PartCoord),
}

fn iter_points(s: PartCoord, e: PartCoord) -> HashSet<PartCoord> {
    let s = s.inner();
    let e = e.inner();
    if s.x == e.x {
        // iterate over y values
        let range = if s.y <= e.y { s.y..=e.y } else { e.y..=s.y };
        range.map(|y| PartCoord::new(IVec2::new(s.x, y))).collect()
    } else {
        // iterate over x values
        let range = if s.x <= e.x { s.x..=e.x } else { e.x..=s.x };
        range.map(|x| PartCoord::new(IVec2::new(x, s.y))).collect()
    }
}

impl PipeGeometry {
    pub fn segments(&self) -> PipeSegments {
        if let Some(b) = get_bend_location(self.start, self.end, self.x_first) {
            PipeSegments::Double(self.start, b, self.end)
        } else {
            PipeSegments::Single(self.start, self.end)
        }
    }

    pub fn points(&self) -> HashSet<PartCoord> {
        match self.segments() {
            PipeSegments::Single(s, e) => iter_points(s, e),
            PipeSegments::Double(s, c, e) => {
                let mut a = iter_points(s, c);
                let b = iter_points(c, e);
                a.extend(&b);
                a
            }
        }
    }

    pub fn with_offset(&self, offset: PartCoord) -> Self {
        Self {
            start: self.start + offset,
            end: self.end + offset,
            x_first: self.x_first,
        }
    }

    pub fn contains(&self, p: PartCoord) -> bool {
        self.points().contains(&p)
    }

    pub fn rotate_ccw(&mut self) {
        self.start = IVec2::Y.rotate(self.start.inner() + IVec2::Y).into();
        self.end = IVec2::Y.rotate(self.end.inner() + IVec2::Y).into();
        self.x_first = !self.x_first;
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

#[cfg(test)]
mod tests {

    use super::*;

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
}
