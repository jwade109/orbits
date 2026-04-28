use bary_core::prelude::*;
use rand::{RngExt, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GridVentory {
    pub slots: Vec<InvSlot>,
    pub pipes: Vec<(usize, usize, MachineStatus)>,
    pub sources: Vec<(usize, Item)>,
    pub sinks: Vec<usize>,
    pub dirty_set: BTreeSet<usize>,
    pub is_settled: bool,
}

impl GridVentory {
    pub fn random(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);

        let n_slots = rng.random_range(200..300);
        let n_pipes = rng.random_range(30..200);

        let slots: Vec<InvSlot> = (0..n_slots)
            .map(|_| {
                let capacity: Volume = Volume::liters(rng.random_range(10..1000));
                let filter = ItemFilter::Any;
                let is_fluid = false;
                let location = (PartCoord::ZERO, PartCoord::ZERO);
                InvSlot::new(capacity, filter, is_fluid, location)
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
                (slot, item)
            })
            .collect();

        let sinks = vec![];

        Self {
            slots,
            pipes,
            sources,
            sinks,
            dirty_set: BTreeSet::new(),
            is_settled: false,
        }
    }

    pub fn mass(&self) -> Mass {
        self.slots.iter().map(|s| s.mass()).sum()
    }
}

pub fn update_inventory(grid: &mut GridVentory) {
    grid.dirty_set.clear();

    for (index, item) in &grid.sources {
        let slot = &mut grid.slots[*index];
        if !slot.is_full() {
            grid.is_settled = false;
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

        grid.dirty_set.insert(*a);
        grid.dirty_set.insert(*b);

        *status = atomic_transfer(src, dst, mass);
    }
}
