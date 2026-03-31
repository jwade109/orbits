use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct PipeJoint {
    pub part_id: Ent,
    pub offset: PartCoord,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Pipe {
    pub src: PipeJoint,
    pub dst: PipeJoint,
}
