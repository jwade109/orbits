use crate::aabb::*;
use crate::math::*;
use crate::parts::*;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

pub fn pixel_dims_with_rotation(rot: Rotation, part: &Part) -> UVec2 {
    let dims = part.dims();
    match rot {
        Rotation::East | Rotation::West => UVec2::new(dims.x, dims.y),
        Rotation::North | Rotation::South => UVec2::new(dims.y, dims.x),
    }
}

fn meters_with_rotation(rot: Rotation, part: &Part) -> Vec2 {
    let w = part.dims_meters();
    match rot {
        Rotation::East | Rotation::West => Vec2::new(w.x, w.y),
        Rotation::North | Rotation::South => Vec2::new(w.y, w.x),
    }
}

#[derive(Debug, Clone, Copy, Sequence, Serialize, Deserialize)]
pub enum Rotation {
    East,
    North,
    West,
    South,
}

#[derive(Debug, Clone)]
pub struct PartInstance {
    builds_remaining: u32,
    origin: IVec2,
    rot: Rotation,
    part: Part,
}

impl PartInstance {
    pub fn new(origin: IVec2, rot: Rotation, part: Part) -> Self {
        // TODO TODO TODO TODO
        Self {
            builds_remaining: 5,
            origin,
            rot,
            part,
        }
    }

    pub fn part(&self) -> &Part {
        &self.part
    }

    pub fn part_mut(&mut self) -> &mut Part {
        &mut self.part
    }

    pub fn build(&mut self) {
        if self.builds_remaining > 0 {
            self.builds_remaining -= 1;
        }
    }

    pub fn percent_built(&self) -> f32 {
        (1.0 - self.builds_remaining as f32 / 5.0).clamp(0.0, 1.0)
    }

    pub fn dims_grid(&self) -> UVec2 {
        pixel_dims_with_rotation(self.rot, &self.part)
    }

    pub fn dims_meters(&self) -> Vec2 {
        meters_with_rotation(self.rot, &self.part)
    }

    pub fn origin(&self) -> IVec2 {
        self.origin
    }

    pub fn set_origin(&mut self, p: IVec2) {
        self.origin = p;
    }

    pub fn obb(&self, angle: f32, scale: f32, pos: Vec2) -> OBB {
        let dims = self.dims_meters();
        let center = rotate(
            self.origin().as_vec2() / crate::prelude::PIXELS_PER_METER + dims / 2.0,
            angle,
        ) * scale;
        OBB::new(
            AABB::from_arbitrary(scale * -dims / 2.0, scale * dims / 2.0),
            angle,
        )
        .offset(center + pos)
    }

    pub fn rotation(&self) -> Rotation {
        self.rot
    }

    pub fn set_rotation(&mut self, rot: Rotation) {
        self.rot = rot;
    }
}
