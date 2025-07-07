use crate::math::*;
use crate::parts::{Rotation, PIXELS_PER_METER};

pub struct InstanceRef<T> {
    pub origin: IVec2,
    pub dims: UVec2,
    pub rot: Rotation,
    pub variant: T,
}

fn rotate_dims(rot: Rotation, part_meters: Vec2) -> Vec2 {
    let w = part_meters;
    match rot {
        Rotation::East | Rotation::West => Vec2::new(w.x, w.y),
        Rotation::North | Rotation::South => Vec2::new(w.y, w.x),
    }
}

impl<T> InstanceRef<T> {
    pub fn new(origin: IVec2, dims: UVec2, rot: Rotation, variant: T) -> Self {
        Self {
            origin,
            dims,
            rot,
            variant,
        }
    }

    pub fn center_meters(&self) -> Vec2 {
        let dims = rotate_dims(self.rot, self.dims.as_vec2() / PIXELS_PER_METER);
        let origin = self.origin.as_vec2() / PIXELS_PER_METER;
        origin + dims / 2.0
    }

    pub fn thrust_pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.rot.to_angle() + PI / 2.0)
    }
}
