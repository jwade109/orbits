use crate::orbits::SparseOrbit;
use crate::propagator::*;
use crate::pv::PV;
use crate::scenario::*;
use crate::{nanotime::Nanotime, orbits::GlobalOrbit};
use serde::{Deserialize, Serialize};

use glam::f32::Vec2;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Orbiter {
    props: Vec<Propagator>,
}

impl std::fmt::Display for Orbiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?} {} props", self.props.first(), self.props.len())
    }
}

impl Orbiter {
    pub fn new(orbit: GlobalOrbit, stamp: Nanotime) -> Self {
        Orbiter {
            props: vec![Propagator::new(orbit, stamp)],
        }
    }

    pub fn try_impulsive_burn(&mut self, stamp: Nanotime, dv: Vec2) -> Option<()> {
        let orbit: GlobalOrbit = {
            let prop = self.propagator_at(stamp)?;
            let pv = prop.pv_universal(stamp)? + PV::vel(dv);
            let orbit = SparseOrbit::from_pv(pv, prop.orbit.1.body, stamp)?;
            GlobalOrbit(prop.parent(), orbit)
        };
        self.props.clear();
        let new_prop = Propagator::new(orbit, stamp);
        self.props.push(new_prop);
        Some(())
    }

    pub fn pv(&self, stamp: Nanotime, planets: &PlanetarySystem) -> Option<PV> {
        let prop = self.propagator_at(stamp)?;
        let (_, pv, _, _) = planets.lookup(prop.parent(), stamp)?;
        Some(prop.pv(stamp)? + pv)
    }

    pub fn pvl(&self, stamp: Nanotime) -> Option<PV> {
        let prop = self.propagator_at(stamp)?;
        Some(prop.pv(stamp)?)
    }

    pub fn propagator_at(&self, stamp: Nanotime) -> Option<&Propagator> {
        self.props.iter().find(|p| p.is_active(stamp))
    }

    pub fn props(&self) -> &Vec<Propagator> {
        &self.props
    }

    pub fn orbit(&self, stamp: Nanotime) -> Option<&GlobalOrbit> {
        let prop = self.propagator_at(stamp)?;
        Some(&prop.orbit)
    }

    pub fn will_collide(&self) -> bool {
        self.props.iter().any(|p| match p.horizon {
            HorizonState::Terminating(_, EventType::Collide(_)) => true,
            _ => false,
        })
    }

    pub fn will_change(&self) -> bool {
        self.props
            .first()
            .map(|p| p.horizon.is_change())
            .unwrap_or(false)
    }

    pub fn is_indefinitely_stable(&self) -> bool {
        self.props.iter().any(|p| p.is_indefinite())
    }

    pub fn has_error(&self) -> bool {
        self.props.iter().any(|p| p.is_err())
    }

    pub fn propagate_to(
        &mut self,
        stamp: Nanotime,
        future_dur: Nanotime,
        planets: &PlanetarySystem,
    ) -> Result<(), PredictError<Nanotime>> {
        while self.props.len() > 1 && self.props[0].end().unwrap_or(stamp) < stamp {
            self.props.remove(0);
        }

        let t = stamp + future_dur;

        let max_iters = 10;

        for _ in 0..max_iters {
            let prop = self.props.iter_mut().last().ok_or(PredictError::Lookup)?;

            let (_, _, _, pl) = planets
                .lookup(prop.parent(), stamp)
                .ok_or(PredictError::Lookup)?;
            let bodies = pl
                .subsystems
                .iter()
                .map(|(orbit, pl)| (pl.id, orbit, pl.body.soi))
                .collect::<Vec<_>>();

            prop.finish_or_compute_until(t, &bodies)?;

            let (end, prop_finished) = match prop.horizon {
                HorizonState::Continuing(end) => (end, false),
                HorizonState::Indefinite => return Ok(()),
                HorizonState::Terminating(_, _) => return Ok(()),
                HorizonState::Transition(end, _) => (end, true),
            };

            if end > t {
                return Ok(());
            }

            if prop_finished {
                match prop.next_prop(planets) {
                    Ok(None) => {
                        return Ok(());
                    }
                    Ok(Some(next)) => {
                        self.props.push(next);
                    }
                    Err(_) => {
                        let mut p = prop.clone();
                        p.start = end;
                        p.horizon = HorizonState::Terminating(end, EventType::NumericalError);
                        self.props.push(p);
                        return Ok(());
                    }
                }
            }
        }

        Err(PredictError::TooManyIterations)
    }
}
