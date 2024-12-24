use crate::core::*;
use bevy::math::Vec2;

pub struct Controller {
    pub id: ObjectId,
}

impl Controller {
    pub fn control(&self, system: &OrbitalSystem, object: &Object) -> Option<Vec2> {
        if object.id != self.id {
            return None;
        }

        let target_radius = 1240.0;
        let tol = 20.0;

        let (_, mass) = system.barycenter();

        let pv = system.global_transform(&object.prop)?;
        let orbit = Orbit::from_pv(pv.pos, pv.vel, mass);

        dbg!(orbit);

        let per = rotate(Vec2::X, 0.0);
        let apo = rotate(Vec2::X, std::f32::consts::PI);
        let cur = rotate(Vec2::X, orbit.true_anomaly);

        if orbit.eccentricity > 0.95 {
            println!("Capturing");
            return Some(-orbit.prograde() * 5.0);
        }

        if cur.dot(apo) > 0.95 {
            // near apoapsis
            if orbit.periapsis().length() < target_radius - tol {
                println!("Raising periapsis");
                return Some(orbit.prograde() * 2.0);
            }
            if orbit.periapsis().length() > target_radius + tol {
                println!("Lowering periapsis");
                return Some(-orbit.prograde() * 2.0);
            }
        }

        if cur.dot(per) > 0.95 {
            // near periapsis
            if orbit.apoapsis().length() > target_radius + tol {
                println!("Lowering apoapsis");
                return Some(-orbit.prograde() * 2.0);
            }

            if orbit.apoapsis().length() < target_radius - tol {
                println!("Raising apoapsis");
                return Some(orbit.prograde() * 2.0);
            }
        }

        None
    }
}
