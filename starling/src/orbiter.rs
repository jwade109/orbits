use crate::core::*;
use crate::orbit::*;
use crate::planning::*;
use crate::pv::PV;

use bevy::math::Vec2;

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
pub struct Orbiter {
    pub id: ObjectId,
    props: Vec<Propagator>,
}

#[derive(Debug, Clone)]
pub enum BadObjectNextState {
    Lookup,
    Removed,
    NoNextState,
    BadOrbit,
    Err,
}

impl Orbiter {
    pub fn new(id: ObjectId, parent: ObjectId, orbit: Orbit, stamp: Nanotime) -> Self {
        Orbiter {
            id,
            props: vec![Propagator::new(parent, orbit, stamp)],
        }
    }

    pub fn dv(&mut self, stamp: Nanotime, dv: Vec2) -> Option<()> {
        let (new_orbit, parent) = {
            let prop = self.propagator_at(stamp)?;
            let pv = prop.orbit.pv_at_time(stamp) + PV::vel(dv);
            let orbit = Orbit::from_pv(pv, prop.orbit.body, stamp)?;
            (orbit, prop.parent)
        };
        self.props.clear();
        let new_prop = Propagator::new(parent, new_orbit, stamp);
        self.props.push(new_prop);
        Some(())
    }

    pub fn pv(&self, stamp: Nanotime, planets: &PlanetarySystem) -> Option<PV> {
        let prop = self.propagator_at(stamp)?;
        let (_, pv, _, _) = planets.lookup(prop.parent, stamp)?;
        Some(prop.orbit.pv_at_time(stamp) + pv)
    }

    pub fn pvl(&self, stamp: Nanotime) -> Option<PV> {
        let prop = self.propagator_at(stamp)?;
        Some(prop.orbit.pv_at_time(stamp))
    }

    pub fn propagator_at(&self, stamp: Nanotime) -> Option<&Propagator> {
        self.props.iter().find(|p| p.is_active(stamp))
    }

    pub fn props(&self) -> &Vec<Propagator> {
        &self.props
    }

    pub fn will_collide(&self) -> bool {
        self.props.iter().any(|p| match p.event {
            Some(EventType::Collide(_)) => true,
            _ => false,
        })
    }

    pub fn has_error(&self) -> bool {
        self.props
            .iter()
            .any(|p| p.event == Some(EventType::NumericalError))
    }

    pub fn propagate_to(
        &mut self,
        stamp: Nanotime,
        future_dur: Nanotime,
        planets: &PlanetarySystem,
    ) -> Result<(), PredictError<Nanotime>> {
        while self.props.len() > 1 && self.props[0].end < stamp {
            self.props.remove(0);
        }

        let t = stamp + future_dur;

        let max_iters = 500;

        for _ in 0..max_iters {
            let prop = self.props.iter_mut().last().ok_or(PredictError::Lookup)?;

            let (_, _, _, pl) = planets
                .lookup(prop.parent, stamp)
                .ok_or(PredictError::Lookup)?;
            let bodies = pl
                .subsystems
                .iter()
                .map(|(orbit, pl)| (pl.id, orbit, pl.body.soi))
                .collect::<Vec<_>>();

            while !prop.calculated_to(t) {
                prop.next(&bodies)?;
            }

            if prop.end >= t {
                return Ok(());
            }

            if prop.finished {
                match prop.next_prop(planets) {
                    Ok(None) => {
                        return Ok(());
                    }
                    Ok(Some(next)) => {
                        self.props.push(next);
                    }
                    Err(_) => {
                        let mut p = prop.clone();
                        p.start = prop.end;
                        p.end = prop.end;
                        p.finished = true;
                        p.event = Some(EventType::NumericalError);
                        self.props.push(p);
                        return Ok(());
                    }
                }
            }
        }

        Err(PredictError::TooManyIterations)
    }
}
