use crate::aabb::AABB;
use crate::nanotime::Nanotime;
use glam::f32::Vec2;
use glam::i32::IVec2;
use std::collections::HashMap;

type BucketId = IVec2;

#[derive(Debug, Clone)]
pub struct TopoMap {
    pub last_updated: Nanotime,
    pub stepsize: f32,
    pub bins: HashMap<BucketId, TopoBin>,
}

impl TopoMap {
    fn new(stepsize: f32) -> Self {
        TopoMap {
            last_updated: Nanotime::zero(),
            stepsize,
            bins: HashMap::new(),
        }
    }

    pub fn update(&mut self, stamp: Nanotime, scalar: &impl Fn(Vec2) -> f32, levels: &[f32]) {
        self.last_updated = stamp;
        for (id, bin) in &mut self.bins {
            bin.update(*id, scalar, levels, self.stepsize)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TopoBin {
    pub contours: Vec<Vec<Vec2>>,
}

fn bucket_id_to_center(id: BucketId, stepsize: f32) -> Vec2 {
    Vec2::new(id.x as f32 * stepsize, id.y as f32 * stepsize)
}

pub fn id_to_aabb(id: BucketId, stepsize: f32) -> AABB {
    let c = bucket_id_to_center(id, stepsize);
    AABB::new(c, Vec2::splat(stepsize))
}

impl TopoBin {
    fn new() -> Self {
        TopoBin {
            contours: Vec::new(),
        }
    }

    fn update(
        &mut self,
        id: BucketId,
        scalar: &impl Fn(Vec2) -> f32,
        levels: &[f32],
        stepsize: f32,
    ) {
        let center = bucket_id_to_center(id, stepsize);
        let bl = center + Vec2::new(-stepsize / 2.0, -stepsize / 2.0);
        let br = center + Vec2::new(stepsize / 2.0, -stepsize / 2.0);
        let tl = center + Vec2::new(-stepsize / 2.0, stepsize / 2.0);
        let tr = center + Vec2::new(stepsize / 2.0, stepsize / 2.0);

        let pot: Vec<(Vec2, f32)> = [bl, br, tr, tl].iter().map(|p| (*p, scalar(*p))).collect();

        // if id == IVec2::new(2, 3) {
        //     dbg!([bl, br, tr, tl]);
        //     dbg!(&pot);
        // }

        self.contours.clear();

        for level in levels {
            let mut pts = vec![];

            for i in 0..4 {
                let p1 = pot[i].0;
                let z1 = pot[i].1;
                let p2 = pot[(i + 1) % 4].0;
                let z2 = pot[(i + 1) % 4].1;

                let l = *level as f32;

                if z1 > l && z2 < l || z1 < l && z2 > l {
                    let t = (l - z1) / (z2 - z1);
                    let d = p1.lerp(p2, t);
                    pts.push(d);
                }
            }
            self.contours.push(pts);
        }
    }
}

pub fn test_topo() -> TopoMap {
    let stepsize = 40.0;
    let scalar = |p: Vec2| p.length();
    let levels = [5.0, 10.0, 30.0, 90.0, 150.0, 200.0];
    let mut tm = TopoMap::new(stepsize);

    for x in -100..=100 {
        for y in -100..100 {
            let c = IVec2::new(x, y);
            let tb = TopoBin::new();
            tm.bins.insert(c, tb);
        }
    }

    tm.update(Nanotime::zero(), &scalar, &levels);

    tm
}
