use crate::canonical::*;
use crate::orbit::*;
use crate::propagator::*;
use bevy::math::Vec2;
use rand::Rng;
use std::collections::VecDeque;
use std::ops::Add;
use std::time::Duration;

pub fn rand(min: f32, max: f32) -> f32 {
    rand::thread_rng().gen_range(min..max)
}

pub fn randvec(min: f32, max: f32) -> Vec2 {
    let rot = Vec2::from_angle(rand(0.0, std::f32::consts::PI * 2.0));
    let mag = rand(min, max);
    rot.rotate(Vec2::new(mag, 0.0))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectId(pub i64);

#[derive(Debug, Clone)]
pub struct Object {
    pub id: ObjectId,
    // pub primary: Option<ObjectId>,
    // pub prop: Propagator,
    pub body: Option<Body>,
    pub history: PropagatorBuffer,
}

impl Object {
    pub fn new(id: ObjectId, prop: impl Into<Propagator>, body: Option<Body>) -> Self {
        Object {
            id,
            // primary: None,
            // prop: prop.into(),
            body,
            history: PropagatorBuffer(VecDeque::from([prop.into()])),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrbitalSystem {
    pub epoch: Duration,
    pub objects: Vec<Object>,
    next_id: i64,
}

pub struct OrbitalFrame {
    pub epoch: Duration,
    pub objects: Vec<(ObjectId, PV, Option<Body>)>,
}

impl OrbitalFrame {
    pub fn lookup(&self, o: ObjectId) -> Option<(ObjectId, PV, Option<Body>)> {
        self.objects.iter().find(|(id, _, _)| *id == o).map(|m| *m)
    }

    pub fn bodies(&self) -> Vec<(ObjectId, PV, Body)> {
        self.objects
            .iter()
            .filter_map(|(id, pv, body)| Some((*id, *pv, (*body)?)))
            .collect()
    }

    pub fn barycenter(&self) -> Vec2 {
        let bodies = self.bodies();
        let total_mass: f32 = bodies.iter().map(|(_, _, b)| b.mass).sum();
        bodies.iter().map(|(_, p, b)| p.pos * b.mass).sum::<Vec2>() / total_mass
    }

    pub fn gravity_at(&self, pos: Vec2) -> Vec2 {
        self.bodies()
            .iter()
            .map(|(_, c, b)| gravity_accel(*b, c.pos, pos))
            .sum()
    }

    pub fn potential_at(&self, pos: Vec2) -> f32 {
        self.bodies()
            .iter()
            .map(|(_, c, b)| {
                let r = (c.pos - pos).length();
                if r < b.radius {
                    return 0.0;
                }
                -b.mu() / r
            })
            .sum()
    }

    pub fn primary_body_at(&self, pos: Vec2, exclude: Option<ObjectId>) -> Option<ObjectId> {
        let mut ret = self
            .bodies()
            .into_iter()
            .filter_map(|(id, pv, body)| {
                if Some(id) == exclude {
                    return None;
                }
                let d = pv.pos.distance(pos);
                if d > body.soi {
                    return None;
                }
                Some((id, body.soi))
            })
            .collect::<Vec<_>>();

        ret.sort_by(|(_, l), (_, r)| l.partial_cmp(r).unwrap());
        ret.first().map(|(o, _)| *o)
    }
}

impl Default for OrbitalSystem {
    fn default() -> Self {
        OrbitalSystem {
            epoch: Duration::default(),
            objects: Vec::default(),
            next_id: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PV {
    pub pos: Vec2,
    pub vel: Vec2,
}

impl PV {
    pub fn new(pos: impl Into<Vec2>, vel: impl Into<Vec2>) -> Self {
        PV {
            pos: pos.into(),
            vel: vel.into(),
        }
    }

    pub fn lerp(&self, other: &Self, s: f32) -> Self {
        PV {
            pos: self.pos.lerp(other.pos, s),
            vel: self.vel.lerp(other.vel, s),
        }
    }
}

impl Add for PV {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        PV::new(self.pos + other.pos, self.vel + other.vel)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OrbitalEvent {
    LookupFailure(ObjectId),
    NumericalError(ObjectId),
    Collision(Vec2, ObjectId, Option<ObjectId>),
    Escaped(Vec2, ObjectId),
}

impl OrbitalSystem {
    pub fn add_object(&mut self, prop: impl Into<Propagator>, body: Option<Body>) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        self.objects.push(Object::new(id, prop, body));
        id
    }

    pub fn current_frame(&self) -> OrbitalFrame {
        self.frame(self.epoch)
    }

    pub fn frame(&self, stamp: Duration) -> OrbitalFrame {
        OrbitalFrame {
            epoch: self.epoch,
            objects: self
                .objects
                .iter()
                .filter_map(|o| Some((o.id, self.global_transform_at(o, stamp)?, o.body)))
                .collect(),
        }
    }

    pub fn has_object(&self, id: ObjectId) -> bool {
        self.objects.iter().find(|o| o.id == id).is_some()
    }

    pub fn min_id(&self) -> Option<ObjectId> {
        self.objects.iter().map(|o| o.id).min()
    }

    pub fn max_id(&self) -> Option<ObjectId> {
        self.objects.iter().map(|o| o.id).max()
    }

    pub fn lookup(&self, o: ObjectId) -> Option<Object> {
        self.objects.iter().find(|m| m.id == o).map(|m| m.clone())
    }

    pub fn lookup_ref(&self, o: ObjectId) -> Option<&Object> {
        self.objects.iter().find(|m| m.id == o)
    }

    pub fn lookup_mut(&mut self, o: ObjectId) -> Option<&mut Object> {
        self.objects.iter_mut().find(|m| m.id == o)
    }

    fn global_transform_at(&self, object: &Object, stamp: Duration) -> Option<PV> {
        let (pv, relopt) = object.history.pv_at(stamp)?;
        if let Some(rel) = relopt {
            let obj = self.lookup_ref(rel)?;
            let rel = self.global_transform_at(obj, stamp)?;
            Some(pv + rel)
        } else {
            Some(pv)
        }
    }

    pub fn propagate_to(&mut self, epoch: Duration) -> Vec<(Object, OrbitalEvent)> {
        let copy = self.frame(self.epoch);
        for m in self.objects.iter_mut() {
            while m.history.0.back().unwrap().epoch() < epoch {
                let old_prop = m.history.0.back().expect("Empty history").clone();
                let new_prop = old_prop.next(&copy, m.id);
                if !new_prop.is_ok() {
                    dbg!(old_prop);
                    dbg!(new_prop);
                    panic!();
                }
                m.history.0.push_back(new_prop);
                if m.history.0.len() > 20 {
                    m.history.0.pop_front();
                }
            }
        }

        self.epoch = epoch;

        // self.reparent_patched_conics();

        let bodies = copy.bodies();

        let remove_with_reason = |o: &Object| -> Option<OrbitalEvent> {
            for prop in o.history.0.iter() {
                if !prop.is_ok() {
                    return Some(OrbitalEvent::NumericalError(o.id));
                }
            }

            let gp = match copy.objects.iter().find(|(oid, _, _)| *oid == o.id) {
                Some((_, pv, _)) => pv,
                None => return Some(OrbitalEvent::LookupFailure(o.id)),
            };

            if gp.pos.length_squared() > 20000.0 * 20000.0 {
                return Some(OrbitalEvent::Escaped(gp.pos, o.id));
            }

            let mut collided = bodies.iter().filter(|(b, pv, body)| {
                let d = pv.pos.distance_squared(gp.pos);
                d != 0.0 && d < body.radius.powi(2)
            });

            if let Some((id, pv, _)) = collided.next() {
                let delta: Vec2 = gp.pos - pv.pos;
                return Some(OrbitalEvent::Collision(delta, o.id, Some(*id)));
            }

            None
        };

        let to_remove = self
            .objects
            .iter()
            .filter_map(|o| match remove_with_reason(o) {
                Some(r) => Some((o.clone(), r)),
                None => None,
            })
            .collect::<Vec<_>>();

        let ids_to_remove = to_remove.iter().map(|(o, _)| o.id).collect::<Vec<_>>();

        self.objects.retain(|o| !ids_to_remove.contains(&o.id));

        to_remove
    }

    // pub fn reparent_patched_conics(&mut self) {
    //     let new_kepler: Vec<_> = self
    //         .objects
    //         .iter()
    //         .filter_map(|obj| {
    //             match &obj.prop {
    //                 Propagator::Kepler(k) => {
    //                     let child_pv = self.global_transform(&obj.prop)?;
    //                     let primary = self.primary_body_at(child_pv.pos, Some(obj.id))?;
    //                     if primary.id == k.primary {
    //                         return None;
    //                     }
    //                     let primary_pv = self.global_transform(&primary.prop)?;
    //                     // TODO math operators for PV?
    //                     let ds = child_pv.pos - primary_pv.pos;
    //                     let dv = child_pv.vel - primary_pv.vel;
    //                     let orbit = Orbit::from_pv(ds, dv, primary.body?);
    //                     let mut new_prop = *k;
    //                     new_prop.orbit = orbit;
    //                     new_prop.primary = primary.id;
    //                     Some((obj.id, new_prop))
    //                 }
    //                 _ => None,
    //             }
    //         })
    //         .collect();

    //     for (id, prop) in new_kepler.iter() {
    //         if let Some(obj) = self.lookup_mut(*id) {
    //             obj.prop = (*prop).into();
    //         }
    //     }
    // }
}

pub fn generate_square_lattice(center: Vec2, w: i32, step: usize) -> Vec<Vec2> {
    let mut ret = vec![];
    for x in (-w..w).step_by(step) {
        for y in (-w..w).step_by(step) {
            ret.push(center + Vec2::new(x as f32, y as f32));
        }
    }
    ret
}

pub fn generate_circular_log_lattice(center: Vec2, rmin: f32, rmax: f32) -> Vec<Vec2> {
    // this isn't actually log, but I'm lazy
    let mut ret = vec![];

    let mut r = rmin;
    let mut dr = 30.0;

    while r < rmax {
        let circ = 2.0 * std::f32::consts::PI * r;
        let mut pts = (circ / dr).ceil() as u32;
        while pts % 8 > 0 {
            pts += 1; // yeah this is stupid
        }
        for i in 0..pts {
            let a = 2.0 * std::f32::consts::PI * i as f32 / pts as f32;
            let x = a.cos();
            let y = a.sin();
            ret.push(center + Vec2::new(x, y) * r);
        }

        r += dr;
        dr *= 1.1;
    }

    ret
}
