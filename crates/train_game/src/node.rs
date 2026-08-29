use crate::{terrain::*, track::*, world::World};
use bary_core::prelude::{Ent, Isometry2d};
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeRole {
    Semantic(Terminus),
    Geometric,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Node {
    pos: DVec2,
    angle: f64,
    pub tracks: BTreeMap<Ent, NodeRole>,
    pub forward_connections: BTreeMap<Ent, Terminus>,
    pub backward_connections: BTreeMap<Ent, Terminus>,
}

impl Node {
    pub fn new(pos: DVec2) -> Self {
        Self {
            pos,
            angle: 0.0,
            tracks: BTreeMap::new(),
            forward_connections: BTreeMap::new(),
            backward_connections: BTreeMap::new(),
        }
    }

    pub fn pos(&self) -> DVec2 {
        self.pos
    }

    pub fn set_pos(&mut self, p: DVec2) {
        self.pos = p;
    }

    pub fn angle(&self) -> f64 {
        self.angle
    }

    pub fn set_angle(&mut self, a: f64) {
        self.angle = a;
    }

    pub fn isometry(&self) -> Isometry2d {
        Isometry2d::new(self.pos.as_vec2(), self.angle as f32)
    }

    pub fn is_semantic(&self) -> bool {
        self.tracks
            .iter()
            .any(|t| matches!(t.1, NodeRole::Semantic(_)))
    }

    pub fn linked_tracks(&self) -> impl Iterator<Item = &Ent> {
        self.tracks
            .iter()
            .filter_map(|(id, role)| matches!(role, NodeRole::Semantic(_)).then_some(id))
    }

    pub fn is_switch(&self) -> bool {
        self.linked_tracks().count() > 2
    }
}

pub fn spawn_new_node(world: &mut World, pos: DVec2) -> Ent {
    let node = Node::new(pos);
    let id = world.spawner.spawn();
    chunk_register_node(world, get_chunk_index(pos), id);
    world.nodes.spawn(id, node);
    id
}

pub fn despawn_node(world: &mut World, id: Ent) {
    if let Ok(node) = world.nodes.despawn(id) {
        chunk_deregister_node(world, get_chunk_index(node.pos), id);
        for (id, _role) in node.tracks {
            _ = despawn_track(world, id);
        }
    }
}
