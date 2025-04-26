use std::collections::HashMap;

use crate::orbiter::OrbiterId;
use crate::orbits::GlobalOrbit;
use crate::pv::PV;
use crate::vehicle::Vehicle;

pub struct RPO {
    orbit: GlobalOrbit,
    vehicles: HashMap<OrbiterId, (PV, Vehicle)>,
}

impl RPO {
    pub fn new(orbit: GlobalOrbit) -> Self {
        Self {
            orbit,
            vehicles: HashMap::new(),
        }
    }
}
