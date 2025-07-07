use crate::aabb::*;
use crate::math::*;
use crate::parts::*;

pub fn pixel_dims_with_rotation(rot: Rotation, part: &Part) -> UVec2 {
    let dims = part.dims();
    match rot {
        Rotation::East | Rotation::West => UVec2::new(dims.x, dims.y),
        Rotation::North | Rotation::South => UVec2::new(dims.y, dims.x),
    }
}

pub fn meters_with_rotation(rot: Rotation, part: &Part) -> Vec2 {
    let w = part.dims_meters();
    match rot {
        Rotation::East | Rotation::West => Vec2::new(w.x, w.y),
        Rotation::North | Rotation::South => Vec2::new(w.y, w.x),
    }
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

    pub fn with_origin(&self, p: IVec2) -> Self {
        let mut ret = self.clone();
        ret.set_origin(p);
        ret
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

    pub fn rotated(&self) -> PartInstance {
        let mut ret = self.clone();
        let old_half_dims = ret.dims_grid().as_vec2() / 2.0;
        let old_center = ret.origin().as_vec2() + old_half_dims;
        let new_center = rotate(old_center, PI / 2.0);
        ret.set_rotation(enum_iterator::next_cycle(&ret.rotation()));
        let new_half_dims = ret.dims_grid().as_vec2() / 2.0;
        let new_corner = new_center - new_half_dims;
        ret.set_origin(vround(new_corner));
        ret
    }

    pub fn as_tank(&self) -> Option<&Tank> {
        if let Part::Tank(t) = &self.part {
            Some(t)
        } else {
            None
        }
    }

    pub fn as_machine(&self) -> Option<&Machine> {
        if let Part::Machine(m) = &self.part {
            Some(m)
        } else {
            None
        }
    }
}
