use crate::{bezier::BezierCurve, world::World};
use bary_core::prelude::{BaryResult, Ent};
use glam::DVec2;

pub struct Node {
    pub pos: DVec2,
}

pub enum SegmentGeometry {
    Straight(Ent, Ent),
    Bezier(Ent, Ent, Ent),
}

pub struct TrackSegment {
    pub geometry: SegmentGeometry,
}

impl TrackSegment {
    pub fn contains_node(&self, id: Ent) -> bool {
        match self.geometry {
            SegmentGeometry::Bezier(a, b, c) => a == id || b == id || c == id,
            SegmentGeometry::Straight(a, b) => a == id || b == id,
        }
    }
}

pub fn spawn_new_node(world: &mut World, pos: DVec2) -> Ent {
    let node = Node { pos };
    let id = world.spawner.spawn();
    world.nodes.spawn(id, node);
    id
}

pub fn remove_node(world: &mut World, id: Ent) -> BaryResult<()> {
    world.nodes.despawn(id)?;
    world.segments.retain(|_, e| !e.contains_node(id));
    Ok(())
}

pub fn spawn_new_track(world: &mut World, points: &[Ent]) -> Option<Ent> {
    let geometry = match points.len() {
        2 => SegmentGeometry::Straight(points[0], points[1]),
        3 => SegmentGeometry::Bezier(points[0], points[1], points[2]),
        _ => {
            return None;
        }
    };

    let track = TrackSegment { geometry };

    let id = world.spawner.spawn();
    world.segments.spawn(id, track);

    Some(id)
}
