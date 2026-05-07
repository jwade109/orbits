use bary_core::prelude::*;
use early_returns::{ok_or_continue, some_or_continue};
use log::warn;
use rand::{RngExt, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    constants::TICKS_PER_SECOND,
    sim::{Components, proto_by_name},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum NewPipeGeometry {
    Straight,
    XFirst,
    YFirst,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GridPipe {
    pub src_coord: PartCoord,
    pub dst_coord: PartCoord,
    pub src: usize,
    pub dst: usize,
    pub status: MachineStatus,
    pub geo: NewPipeGeometry,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GridVentory {
    pub slot_ids: Vec<Ent>,
    pub slots: Vec<InvSlot>,

    #[serde(skip)]
    pub slot_map: BTreeMap<PartCoord, usize>,

    #[serde(skip)]
    pub pipe_map: BTreeMap<PartCoord, usize>,

    pub pipe_ids: Vec<Ent>,
    pub pipes: Vec<GridPipe>,

    pub roi: Vec<GridRegion>,

    pub sources: Vec<(usize, u64, Item)>,
    pub sinks: Vec<usize>,
    // pub dirty_set: BTreeSet<usize>,
    pub is_settled: bool,
}

impl GridVentory {
    pub fn from_blueprint(bp: &Blueprint, protos: &Components<PartPrototype>) -> Self {
        let mut s = Self::default();

        for (_, part) in bp.parts() {
            if part.layer() != PartLayer::Internal {
                continue;
            }

            let proto_id = some_or_continue!(proto_by_name(protos, &part.name));
            let proto = ok_or_continue!(protos.try_get(proto_id));

            if let Some(data) = &proto.inventory_data {
                s.roi.push(part.region);
                for slot in &data.slots {
                    let dims = slot.max - slot.min;
                    let slot_tf = GridIsometry2d::new(slot.min, Rotation::East);
                    let part_tf = part.region.discrete_transform();
                    let combined_tf = part_tf * slot_tf;
                    let u = combined_tf.translation;
                    let v = u + combined_tf.local_x() * dims.x + combined_tf.local_y() * dims.y;
                    let min: PartCoord = (u.x.min(v.x), u.y.min(v.y)).into();
                    let max: PartCoord = (u.x.max(v.x), u.y.max(v.y)).into();
                    let success = s.add_slot(Volume::liters(1000), min, max);
                    if success.is_none() {
                        warn!("{:?} {} {}", slot.name, max - min, success.is_some());
                    }
                }
            }
        }

        for (_, hose) in bp.pipes() {
            s.add_pipe_at(hose.start, hose.end, NewPipeGeometry::Straight);
        }

        s
    }

    pub fn random(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);

        let n_slots = rng.random_range(20..50);
        let n_pipes = rng.random_range(10..40);

        let mut ent = Ent(100);

        let slot_ids = (0..n_slots)
            .map(|_| {
                let id = ent;
                ent.0 += 1;
                id
            })
            .collect();

        let slots: Vec<InvSlot> = (0..n_slots)
            .map(|i| {
                let capacity: Volume = Volume::liters(rng.random_range(10..1000));
                let filter = ItemFilter::Any;
                let is_fluid = false;
                let x = i % 10;
                let y = i / 10;
                let lower = PartCoord::new((x, y));
                let upper = lower + PartCoord::ONE;
                let location = (lower, upper);
                InvSlot::new(capacity, filter, is_fluid, location)
            })
            .collect();

        let sources = (0..5)
            .map(|_| {
                let item = Item::random();
                let slot = rng.random_range(0..slots.len() - 1);
                (slot, 1000 / TICKS_PER_SECOND, item)
            })
            .collect();

        let sinks = vec![];

        let mut grid = Self {
            slot_ids,
            slots,
            sources,
            sinks,
            ..Default::default()
        };

        for _ in 0..n_pipes {
            let x0 = rng.random_range(0..10);
            let y0 = rng.random_range(0..10);
            let xf = rng.random_range(0..10);
            let yf = rng.random_range(0..10);
            grid.add_pipe_at((x0, y0), (xf, yf), NewPipeGeometry::Straight);
        }

        grid
    }

    pub fn mass(&self) -> Mass {
        self.slots.iter().map(|s| s.mass()).sum()
    }

    pub fn add_slot(
        &mut self,
        capacity: Volume,
        min: impl Into<PartCoord>,
        max: impl Into<PartCoord>,
    ) -> Option<usize> {
        let filter = ItemFilter::Any;
        let is_fluid = false;
        let location = (min.into(), max.into());
        let slot = InvSlot::new(capacity, filter, is_fluid, location);
        let id = self.slots.len();

        for x in location.0.inner().x..location.1.inner().x {
            for y in location.0.inner().y..location.1.inner().y {
                let c = PartCoord::new((x, y));
                if self.slot_map.contains_key(&c) {
                    return None;
                }
            }
        }

        for x in location.0.inner().x..location.1.inner().x {
            for y in location.0.inner().y..location.1.inner().y {
                let c = PartCoord::new((x, y));
                self.slot_map.insert(c, id);
            }
        }

        self.slots.push(slot);

        Some(id)
    }

    pub fn add_source(&mut self, a: usize) {
        self.sources
            .push((a, 1000 / TICKS_PER_SECOND, Item::random()));
    }

    pub fn add_sink(&mut self, a: usize) {
        self.sinks.push(a);
    }

    pub fn slot_at(&self, c: PartCoord) -> Option<usize> {
        self.slot_map.get(&c).copied()
    }

    pub fn add_pipe_at(
        &mut self,
        a: impl Into<PartCoord>,
        b: impl Into<PartCoord>,
        geo: NewPipeGeometry,
    ) -> Option<()> {
        let a = a.into();
        let b = b.into();

        if self.pipe_map.contains_key(&a) || self.pipe_map.contains_key(&b) {
            // return None;
            warn!("Accepting colliding pipes");
        }

        if a == b {
            return None;
        }
        let src = self.slot_at(a)?;
        let dst = self.slot_at(b)?;
        if src == dst {
            return None;
        }

        let id = self.pipes.len();

        self.pipe_map.insert(a, id);
        self.pipe_map.insert(b, id);

        let pipe = GridPipe {
            src,
            dst,
            src_coord: a,
            dst_coord: b,
            status: MachineStatus::Off,
            geo,
        };
        self.pipes.push(pipe);
        Some(())
    }
}

pub fn update_inventory(grid: &mut GridVentory) {
    // grid.dirty_set.clear();

    for (index, count, item) in &grid.sources {
        let slot = &mut grid.slots[*index];
        if !slot.is_full() {
            grid.is_settled = false;
            // slot.store_partial(*item, *count);
            slot.fill_with(*item);
        }
    }

    if grid.is_settled {
        return;
    }

    grid.is_settled = true;

    for index in &grid.sinks {
        let slot = &mut grid.slots[*index];
        slot.empty();
    }

    for pipe in &mut grid.pipes {
        let [src, dst] = grid.slots.get_disjoint_mut([pipe.src, pipe.dst]).unwrap();

        if src.is_empty() || dst.is_full() {
            pipe.status = MachineStatus::Off;
            continue;
        }

        if src.item().is_some() && dst.item().is_some() && src.item() != dst.item() {
            pipe.status = MachineStatus::Off;
            continue;
        }

        grid.is_settled = false;

        let mass = {
            let mul = 150;
            let m = src.mass() / mul as u64;
            if m.is_zero() { Mass::grams(1) } else { m }
        };

        pipe.status = atomic_transfer(src, dst, mass);
    }
}
