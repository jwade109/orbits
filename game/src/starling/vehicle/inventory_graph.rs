use crate::prelude::*;
use crate::starling::prelude::*;

use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Copy)]
enum NodeType {
    Inventory,
    Thruster,
    Machine,
    DockingPort,
}

#[derive(Debug, Clone, Copy)]
struct GraphNode {
    pos: Vec2,
    vel: Vec2,
    real_pos: PartCoord,
    id: PartId,
    slot_id: u32,
    node_type: NodeType,
}

impl GraphNode {
    fn update(&mut self, other: &Self) {
        self.id = other.id;
        self.slot_id = other.slot_id;
        self.node_type = other.node_type;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeKey(pub PartId);

#[derive(Default, Debug)]
pub struct InventoryGraph {
    nodes: BTreeMap<NodeKey, GraphNode>,
    edges: HashSet<(NodeKey, NodeKey)>,
}

impl InventoryGraph {
    pub fn from_blueprint(bp: &Blueprint) -> Self {
        let mut nodes = BTreeMap::new();
        let mut edges = HashSet::new();
        let mut next_id = 0;
        for (id, part) in bp.parts() {
            let node_type = if part.proto.thruster_data.is_some() {
                NodeType::Thruster
            } else if part.proto.docking_port_data.is_some() {
                NodeType::DockingPort
            } else if part.proto.machine_data.is_some() {
                NodeType::Machine
            } else if part.proto.inventory_data.is_some() {
                NodeType::Inventory
            } else {
                continue;
            };

            let center = part.origin().inner() + part.dims_grid().as_ivec2() / 2;

            let node = GraphNode {
                pos: randvec(10.0, 100.0),
                vel: Vec2::ZERO,
                real_pos: PartCoord::new(center),
                id: *id,
                slot_id: 0,
                node_type,
            };

            let id = NodeKey(*id);

            nodes.insert(id, node);
            next_id += 1;
        }

        for (_, a, b) in bp.pipe_connections() {
            let Some((na, _)) = nodes.iter().find(|(_, n)| n.id == a) else {
                continue;
            };
            let Some((nb, _)) = nodes.iter().find(|(_, n)| n.id == b) else {
                continue;
            };

            edges.insert((*na, *nb));
            edges.insert((*nb, *na));
        }

        Self { nodes, edges }
    }

    pub fn update(&mut self, mut other: InventoryGraph) {
        for (id, node) in &mut other.nodes {
            if let Some(n) = self.nodes.get(id) {
                node.pos = n.pos;
                node.vel = n.vel;
            }
        }

        *self = other;
    }

    pub fn randomize_positions(&mut self) {
        for (_, node) in &mut self.nodes {
            node.pos = randvec(10.0, 200.0);
        }
    }

    pub fn step(&mut self, dt: f32) {
        let mut ids: Vec<_> = self.nodes.keys().cloned().collect();
        ids.sort();

        for i in &ids {
            let Some(node_a) = self.nodes.get(i) else {
                continue;
            };

            let pos_a = node_a.pos;

            let mut repulsion_force = Vec2::ZERO;
            let mut connected_force = Vec2::ZERO;

            const KR: f32 = 9.0;
            const KA: f32 = 1.0;
            const KC: f32 = 15.0;
            const KP: f32 = 4.0;
            const REPULSION_RADIUS: f32 = 400.0;
            const DESIRED_CONN_RADIUS: f32 = 30.0;

            for j in &ids {
                if i == j {
                    continue;
                }

                let Some(node_b) = self.nodes.get(j) else {
                    continue;
                };

                let pos_b = node_b.pos;

                let delta = pos_b - pos_a;

                if delta.length() < REPULSION_RADIUS && delta.length() > 1.0 {
                    let dir = delta.normalize_or_zero();
                    let mag = KR * REPULSION_RADIUS / delta.length();
                    repulsion_force -= dir * mag;
                }

                if self.edges.contains(&(*i, *j)) {
                    let dir = delta.normalize_or_zero();
                    let mag = KC * (delta.length() - DESIRED_CONN_RADIUS);
                    connected_force += dir * mag;
                }
            }

            let Some(node_a) = self.nodes.get_mut(i) else {
                continue;
            };

            let real_pos = node_a.real_pos.to_meters() * 25.0;

            let attraction_force = KA * -pos_a;
            let real_pos_force = KP * (real_pos - pos_a);

            node_a.vel += repulsion_force * dt;
            node_a.vel += attraction_force * dt;
            node_a.vel += connected_force * dt;
            node_a.vel += real_pos_force * dt;
            node_a.pos += node_a.vel * dt;

            node_a.vel *= 0.98;
        }
    }
}

pub fn draw_inventory_graph(canvas: &mut Canvas, graph: &InventoryGraph, offset: Vec2, scale: f32) {
    let z = ZOrdering::Debug2;

    let w2p = |p: Vec2| -> Vec2 { offset + p * scale };

    for (_, node) in &graph.nodes {
        // let p = ctx.w2c(node.pos.as_dvec2());
        let rect = AABB::from_wh(10.0 * scale, 10.0 * scale).with_center(w2p(node.pos));
        let color = match node.node_type {
            NodeType::Inventory => GREEN,
            NodeType::Thruster => ORANGE,
            NodeType::Machine => RED,
            NodeType::DockingPort => WHITE,
        };
        canvas.hollow_rect(rect, z, color, 1.0);
    }

    for (a, b) in &graph.edges {
        let Some(a) = graph.nodes.get(a) else {
            continue;
        };
        let Some(b) = graph.nodes.get(b) else {
            continue;
        };

        canvas.line(w2p(a.pos), w2p(b.pos), z, GREEN.with_alpha(0.4));
    }
}
