use bary_core::prelude::*;
use rand::{RngExt, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};

use crate::constants::TICKS_PER_SECOND;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GridVentory {
    pub slot_ids: Vec<Ent>,
    pub slots: Vec<InvSlot>,

    pub pipe_ids: Vec<Ent>,
    pub pipes: Vec<(usize, usize, MachineStatus)>,

    pub sources: Vec<(usize, u64, Item)>,
    pub sinks: Vec<usize>,
    // pub dirty_set: BTreeSet<usize>,
    pub is_settled: bool,
}

impl GridVentory {
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

        let pipe_ids = (0..n_pipes)
            .map(|_| {
                let id = ent;
                ent.0 += 1;
                id
            })
            .collect();

        let pipes = (0..n_pipes)
            .map(|_| {
                let a = rng.random_range(0..slots.len() - 1);
                let b = rng.random_range(0..slots.len() - 1);
                (a, b, MachineStatus::Running)
            })
            .filter(|(a, b, _)| a != b)
            .collect();

        let sources = (0..5)
            .map(|_| {
                let item = Item::random();
                let slot = rng.random_range(0..slots.len() - 1);
                (slot, 1000 / TICKS_PER_SECOND, item)
            })
            .collect();

        let sinks = vec![];

        Self {
            slot_ids,
            slots,
            pipe_ids,
            pipes,
            sources,
            sinks,
            // dirty_set: BTreeSet::new(),
            is_settled: false,
        }
    }

    pub fn mass(&self) -> Mass {
        self.slots.iter().map(|s| s.mass()).sum()
    }

    pub fn add_slot(&mut self, min: impl Into<PartCoord>, max: impl Into<PartCoord>) -> usize {
        let capacity = Volume::liters(1000);
        let filter = ItemFilter::Any;
        let is_fluid = false;
        let location = (min.into(), max.into());
        let slot = InvSlot::new(capacity, filter, is_fluid, location);
        let id = self.slots.len();
        self.slots.push(slot);
        id
    }

    pub fn add_pipe(&mut self, a: usize, b: usize) {
        self.pipes.push((a, b, MachineStatus::Off));
    }

    pub fn add_source(&mut self, a: usize) {
        self.sources
            .push((a, 1000 / TICKS_PER_SECOND, Item::random()));
    }

    pub fn add_sink(&mut self, a: usize) {
        self.sinks.push(a);
    }
}

pub fn update_inventory(grid: &mut GridVentory) {
    // grid.dirty_set.clear();

    for (index, count, item) in &grid.sources {
        let slot = &mut grid.slots[*index];
        if !slot.is_full() {
            grid.is_settled = false;
            slot.store_partial(*item, *count);
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

    for (a, b, status) in &mut grid.pipes {
        if a == b {
            continue;
        }

        let [src, dst] = grid.slots.get_disjoint_mut([*a, *b]).unwrap();

        if src.is_empty() || dst.is_full() {
            *status = MachineStatus::Off;
            continue;
        }

        if src.item().is_some() && dst.item().is_some() && src.item() != dst.item() {
            *status = MachineStatus::Off;
            continue;
        }

        grid.is_settled = false;

        let mass = {
            let mul = 150;
            let m = src.mass() / mul as u64;
            if m.is_zero() { Mass::grams(1) } else { m }
        };

        *status = atomic_transfer(src, dst, mass);
    }
}
