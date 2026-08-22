use std::collections::BTreeSet;

use crate::world::World;
use bary_core::prelude::Ent;
use glam::DVec2;

pub struct Node {
    pub pos: DVec2,
    pub tracks: BTreeSet<Ent>,
}

impl Node {
    pub fn new(pos: DVec2) -> Self {
        Self {
            pos,
            tracks: BTreeSet::new(),
        }
    }
}

pub struct TrackSegment {
    pub nodes: Vec<Ent>,
}

impl TrackSegment {
    pub fn new(nodes: Vec<Ent>) -> Option<Self> {
        (nodes.len() >= 2).then_some(Self { nodes })
    }

    pub fn end_nodes(&self) -> [Ent; 2] {
        [*self.nodes.first().unwrap(), *self.nodes.last().unwrap()]
    }

    pub fn is_connected_at_end(&self, id: Ent) -> bool {
        let [a, b] = self.end_nodes();
        a == id || b == id
    }
}

pub fn spawn_new_node(world: &mut World, pos: DVec2) -> Ent {
    let node = Node::new(pos);
    let id = world.spawner.spawn();
    world.nodes.spawn(id, node);
    id
}

pub fn despawn_node(world: &mut World, id: Ent) {
    if let Ok(node) = world.nodes.despawn(id) {
        for id in node.tracks {
            _ = despawn_track(world, id);
        }
    }
}

pub fn despawn_track(world: &mut World, track_id: Ent) -> Option<()> {
    let track = world.segments.despawn(track_id).ok()?;
    for id in track.nodes {
        if let Ok(node) = world.nodes.try_get_mut(id) {
            node.tracks.remove(&track_id);
        }
    }
    Some(())
}

pub fn spawn_new_track(world: &mut World, nodes: Vec<Ent>) -> Option<Ent> {
    let track = TrackSegment::new(nodes)?;

    let track_id = world.spawner.spawn();

    for id in &track.nodes {
        let node = world.nodes.try_get_mut(*id).ok()?;
        node.tracks.insert(track_id);
    }

    world.segments.spawn(track_id, track);

    Some(track_id)
}

pub fn visit(world: &World, current: Ent, target: Ent, visited: &mut BTreeSet<Ent>) -> bool {
    if visited.contains(&current) {
        return false;
    }

    visited.insert(current);

    println!("{:?} {:?}", current, visited);

    if current == target {
        println!("Got there! {:?}", visited);
        return true;
    }

    let Some(node) = world.nodes.get(current) else {
        return false;
    };
    for track_id in &node.tracks {
        let Some(track) = world.segments.get(*track_id) else {
            return false;
        };

        if !track.is_connected_at_end(current) {
            return false;
        }

        for node_id in track.end_nodes() {
            if visit(world, node_id, target, visited) {
                return true;
            }
        }
    }

    false
}

pub fn pathfind(world: &World, start: Ent, target: Ent) -> Option<()> {
    let mut visited = BTreeSet::new();
    _ = visit(world, start, target, &mut visited);
    Some(())
}

pub fn spawn_three_way_junction(world: &mut World, a: Ent, b: Ent, c: Ent) -> Option<()> {
    let na = world.nodes.get(a)?;
    let nb = world.nodes.get(b)?;
    let nc = world.nodes.get(c)?;

    let center = (na.pos + nb.pos + nc.pos) / 3.0;

    let d = spawn_new_node(world, center);

    spawn_new_track(world, vec![a, d, b]);
    spawn_new_track(world, vec![a, d, c]);
    spawn_new_track(world, vec![b, d, c]);

    Some(())
}
