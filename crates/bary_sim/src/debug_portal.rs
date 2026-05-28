use bary_factory::*;
use bary_parts::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum PortalState {
    Source(Option<Item>),
    Sink,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct DebugPortal {
    pub state: PortalState,
}

impl DebugPortal {
    pub fn from_proto(proto: DebugPortalPrototype) -> Self {
        let state = match proto.dir {
            PortalDirection::Sink => PortalState::Sink,
            PortalDirection::Source => PortalState::Source(Some(Item::random())),
        };
        Self { state }
    }
}
