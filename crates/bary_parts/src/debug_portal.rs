use bary_factory::Item;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum PortalDirection {
    Source,
    Sink,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct DebugPortalPrototype {
    pub dir: PortalDirection,
}
