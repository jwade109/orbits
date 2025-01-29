use crate::core::*;
use crate::orbit::*;
use crate::planning::*;
use bevy::math::Vec2;
use std::collections::VecDeque;

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

#[derive(Debug, Clone, Copy)]
pub enum Maneuver {
    AxisAligned(Vec2),
}

#[derive(Debug, Clone)]
pub struct Object {
    pub id: ObjectId,
    props: Vec<Propagator>,
    maneuvers: Vec<(Nanotime, Maneuver)>,
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
            props: vec![Propagator::new(parent, orbit, stamp)],
            maneuvers: Vec::new(),
        }
    }

    pub fn propagator_at(&self, stamp: Nanotime) -> Option<&Propagator> {
        self.props.iter().find(|p| p.is_active(stamp))
    }

    pub fn props(&self) -> &Vec<Propagator> {
        &self.props
    }

    pub fn maneuvers(&self) -> &Vec<(Nanotime, Maneuver)> {
        &self.maneuvers
    }

    pub fn add_maneuver(&mut self, stamp: Nanotime, man: Maneuver) -> Option<()> {
        self.maneuvers.push((stamp, man));
        self.maneuvers.sort_by_key(|e| e.0);
        self.props.retain(|p| {
            let fully_after = p.start > stamp;
            !fully_after
        });

        for prop in &mut self.props {
            if prop.start <= stamp && prop.end > stamp {
                prop.end = stamp;
                prop.finished = true;
                prop.event = Some(EventType::Maneuver(man));
            }
        }

        Some(())
    }

    pub fn propagate_to(
        &mut self,
        stamp: Nanotime,
        future_dur: Nanotime,
        planets: &Planet,
    ) -> Option<()> {
        while self.props.len() > 1 && self.props[0].end < stamp {
            self.props.remove(0);
        }

        let prop = self.props.iter().last()?;

        if prop.finished {
            if let Some(next) = prop.next_prop(planets) {
                self.props.push(next);
            }
        }

        let prop = self.props.iter_mut().last()?;

        while !prop.calculated_to(stamp + future_dur) {
            let (_, _, _, pl) = planets.lookup(prop.parent, stamp)?;
            let bodies = pl
                .subsystems
                .iter()
                .map(|(orbit, pl)| (pl.id, *orbit, pl.primary.soi))
                .collect::<Vec<_>>();

            let _ = prop.next(pl.primary.radius, pl.primary.soi, &bodies, &self.maneuvers);
        }

        Some(())
    }

    // pub fn next(&self, planet: &Planet) -> Result<(OrbitalEvent, Object), BadObjectNextState> {
    //     let mut o = self.clone();
    //     let event = o.pop_front().ok_or(BadObjectNextState::NoNextState)?;
    //     match event.etype {
    //         EventType::Maneuver(dv) => {
    //             let (body, _, _, _) = planet
    //                 .lookup(self.prop.parent, event.stamp)
    //                 .ok_or(BadObjectNextState::Lookup)?;
    //             let pv = o.prop.orbit.pv_at_time(event.stamp);
    //             let new_pv = pv + PV::vel(dv);
    //             let orbit = Orbit::from_pv(new_pv, body.mass, event.stamp);
    //             o.prop.orbit = orbit;
    //             o.prop.reset(event.stamp);
    //             Ok((event, o))
    //         }
    //         EventType::Encounter(id) => {
    //             let (new_body, new_pv, _, _) = planet
    //                 .lookup(id, event.stamp)
    //                 .ok_or(BadObjectNextState::Lookup)?;
    //             let (_, old_pv, _, _) = planet
    //                 .lookup(self.prop.parent, event.stamp)
    //                 .ok_or(BadObjectNextState::Lookup)?;
    //             let ego = self.prop.orbit.pv_at_time(event.stamp) + old_pv;
    //             let d = ego - new_pv;
    //             let orbit = Orbit::from_pv(d, new_body.mass, event.stamp);
    //             o.prop.orbit = orbit;
    //             o.prop.parent = id;
    //             o.prop.reset(event.stamp);
    //             Ok((event, o))
    //         }
    //         EventType::Escape => {
    //             let (_, old_frame_pv, reparent_id, _) = planet
    //                 .lookup(self.prop.parent, event.stamp)
    //                 .ok_or(BadObjectNextState::Lookup)?;
    //             let reparent = reparent_id.ok_or(BadObjectNextState::Err)?;
    //             let (new_body, new_frame_pv, _, _) = planet
    //                 .lookup(reparent, event.stamp)
    //                 .ok_or(BadObjectNextState::Lookup)?;
    //             let pv = self.prop.orbit.pv_at_time(event.stamp);
    //             let d = pv + old_frame_pv - new_frame_pv;
    //             let orbit = Orbit::from_pv(d, new_body.mass, event.stamp);
    //             o.prop.orbit = orbit;
    //             o.prop.parent = reparent;
    //             o.prop.reset(event.stamp);
    //             Ok((event, o))
    //         }
    //         EventType::Collide => Err(BadObjectNextState::NoNextState),
    //     }
    // }
}
