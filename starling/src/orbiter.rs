use crate::core::*;
use crate::orbit::*;
use crate::planning::*;
use crate::pv::PV;

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectId(pub i64);

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    pub id: ObjectId,
    pub parent: ObjectId,
    pub orbit: Orbit,
    pub propagator: Propagator,
    pub events: std::collections::VecDeque<OrbitalEvent>,
}

#[derive(Debug, Clone)]
pub enum BadObjectNextState {
    Lookup,
    Removed,
    NoNextState,
    Err,
}

impl Object {
    pub fn new(id: ObjectId, parent: ObjectId, orbit: Orbit, stamp: Nanotime) -> Self {
        Object {
            id,
            parent,
            orbit,
            propagator: Propagator::new(stamp),
            events: std::collections::VecDeque::new(),
        }
    }

    pub fn valid_until(&mut self) -> Nanotime {
        self.propagator.stamp()
    }

    pub fn add_event(&mut self, event: OrbitalEvent) {
        // TODO enforce order and clear future events to recompute!
        self.events.push_back(event);
    }

    pub fn propagate_to(&mut self, stamp: Nanotime, planets: &Planet) {
        while self.valid_until() < stamp && !self.events.is_empty() {
            match self.next(planets) {
                Ok((_, o)) => {
                    *self = o;
                }
                Err(_) => break,
            }
        }
    }

    pub fn next(&self, planet: &Planet) -> Result<(OrbitalEvent, Object), BadObjectNextState> {
        let mut o = self.clone();
        let event = o
            .events
            .pop_front()
            .ok_or(BadObjectNextState::NoNextState)?;
        match event.etype {
            EventType::Maneuver(dv) => {
                let (body, _, _) = planet
                    .lookup(self.parent, event.stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let pv = o.orbit.pv_at_time(event.stamp);
                let new_pv = pv + PV::vel(dv);
                let orbit = Orbit::from_pv(new_pv, body.mass, event.stamp);
                o.orbit = orbit;
                o.propagator.reset(event.stamp);
                Ok((event, o))
            }
            EventType::Encounter(id) => {
                let (new_body, new_pv, _) = planet
                    .lookup(id, event.stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let (_, old_pv, _) = planet
                    .lookup(self.parent, event.stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let ego = self.orbit.pv_at_time(event.stamp) + old_pv;
                let d = ego - new_pv;
                let orbit = Orbit::from_pv(d, new_body.mass, event.stamp);
                o.orbit = orbit;
                o.parent = id;
                o.propagator.reset(event.stamp);
                Ok((event, o))
            }
            EventType::Escape => {
                let (_, old_frame_pv, reparent_id) = planet
                    .lookup(self.parent, event.stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let reparent = reparent_id.ok_or(BadObjectNextState::Err)?;
                let (new_body, new_frame_pv, _) = planet
                    .lookup(reparent, event.stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let pv = self.orbit.pv_at_time(event.stamp);
                let d = pv + old_frame_pv - new_frame_pv;
                let orbit = Orbit::from_pv(d, new_body.mass, event.stamp);
                o.orbit = orbit;
                o.parent = reparent;
                o.propagator.reset(event.stamp);
                Ok((event, o))
            }
            EventType::Collide => Err(BadObjectNextState::NoNextState),
        }
    }
}
