use crate::sim::*;
use bary_core::prelude::*;
use bary_factory::*;
use bary_orbital::*;
use bary_sim::*;
use early_returns::*;
use std::collections::*;

/// Fills inventories which have debug sources attached.
fn sys_fill_inventories_attached_to_debug_sources(world: &mut World) {
    for (part_id, portal) in world.debug_portals.iter() {
        let part = ok_or_continue!(world.parts.try_get(*part_id));
        let loc = GridLocation::new(part.grid_id, part.region.origin());
        let slot = ok_or_continue!(get_slot_mut_c(
            loc,
            &world.grids,
            &world.parts,
            &mut world.inventories
        ));

        match portal.state {
            PortalState::Source(item) => {
                if let Some(item) = item {
                    slot.fill_with(item);
                }
            }
            PortalState::Sink => {
                slot.empty();
            }
        }
    }
}

/// Performs inventory transfers according to the pipes that exist
/// in the world.
fn sys_update_pipes(inventories: &mut Components<Inventory>, pipes: &mut Components<Pipe>) {
    for pipe in pipes.values_mut() {
        let inv_a = ok_or_continue!(inventories.try_get(pipe.src.part_id));
        let inv_b = ok_or_continue!(inventories.try_get(pipe.dst.part_id));

        let mut src = some_or_continue!(inv_a.get_slot(pipe.src.slot)).clone();
        let mut dst = some_or_continue!(inv_b.get_slot(pipe.dst.slot)).clone();

        if src.is_empty() {
            pipe.status = MachineStatus::Starved;
            continue;
        }

        let mass = {
            let mul = randint(140, 160);
            let m = src.mass() / mul as u64;
            if m.is_zero() { Mass::grams(1) } else { m }
        };

        pipe.status = atomic_transfer(&mut src, &mut dst, mass);

        _ = set_inventory_slot(inventories, src, pipe.src.part_id, pipe.src.slot);
        _ = set_inventory_slot(inventories, dst, pipe.dst.part_id, pipe.dst.slot);
    }
}

/// Steps running machines forward by one tick, and modifies their
/// corresponding inventory if necessary.
fn sys_update_machines(world: &mut World) {
    for (part_id, machine) in world.machines.iter_mut() {
        step_process(machine, *part_id, &mut world.inventories);
    }
}

/// Iterates over all active excavators and removes tiles accordingly
fn sys_mine_tiles(world: &mut World) {
    let mut to_remove: BTreeMap<Ent, BTreeSet<GlobalTileIndex>> = BTreeMap::new();
    for (id, ex) in world.excavators.iter() {
        let Ok(Some((ast_id, tiles))) = get_excavator_tiles(*id, ex, world) else {
            continue;
        };
        to_remove
            .entry(ast_id)
            .and_modify(|e| {
                for t in &tiles {
                    e.insert(*t);
                }
            })
            .or_insert(BTreeSet::from_iter(tiles));
    }

    for (ast_id, tiles) in to_remove {
        for t in tiles {
            _ = remove_terrain_tile(world, ast_id, t);
        }
    }
}

// fn keyboard_control_law(keys: &ButtonInput<KeyCode>) -> VehicleControl {
//     let mut ctrl = VehicleControl::NULLOPT;

//     let docking_mode = keys.pressed(KeyCode::ControlLeft);

//     if docking_mode {
//         ctrl.plus_x.throttle = keys.pressed(KeyCode::ArrowUp) as u8 as f32;
//         ctrl.plus_y.throttle = keys.pressed(KeyCode::ArrowRight) as u8 as f32;
//         ctrl.neg_x.throttle = keys.pressed(KeyCode::ArrowDown) as u8 as f32;
//         ctrl.neg_y.throttle = keys.pressed(KeyCode::ArrowLeft) as u8 as f32;
//     } else {
//         ctrl.plus_x.throttle = keys.pressed(KeyCode::ArrowUp) as u8 as f32;
//         ctrl.neg_x.throttle = keys.pressed(KeyCode::ArrowDown) as u8 as f32;

//         ctrl.attitude = if keys.pressed(KeyCode::ArrowLeft) {
//             10.0
//         } else if keys.pressed(KeyCode::ArrowRight) {
//             -10.0
//         } else {
//             0.0
//         };
//     }

//     ctrl.plus_x.use_rcs = docking_mode;
//     ctrl.plus_y.use_rcs = docking_mode;
//     ctrl.neg_x.use_rcs = docking_mode;
//     ctrl.neg_y.use_rcs = docking_mode;

//     ctrl
// }

pub fn update_world(world: &mut World) -> DebugTimers {
    world.ticks += 1;

    let mut timers = DebugTimers::default();
    timers.ticks += 1;

    {
        let _timer = timers.scope("grid_motion");

        sys_update_ring_particles(&mut world.particles, world.ticks);
        let dirty_set = sys_update_thrusters(
            &mut world.thrusters,
            &world.grids,
            &world.parts,
            &world.computers,
        );

        world.grid_acceleration_updates += dirty_set.len() as u64;
        sys_update_grid_acceleration_c(dirty_set, &mut world.grids, &world.thrusters, &world.parts);
        sys_update_computers(&mut world.computers, &world.parts, &world.grids);
        sys_propagate_grid_rigid_bodies(&mut world.grids);
    }

    {
        let _timer = timers.scope("update_trackers");

        sys_update_trackers(&mut world.tracking, &world.grids, world.ticks);
    }

    {
        let _timer = timers.scope("update_pipes");

        sys_update_pipes(&mut world.inventories, &mut world.pipes);
    }

    {
        let _timer = timers.scope("fill_inventories");

        sys_fill_inventories_attached_to_debug_sources(world);
    }

    {
        let _timer = timers.scope("update_machines");

        sys_update_machines(world);
    }

    {
        let _timer = timers.scope("terrain_mining");

        sys_mine_tiles(world);
    }

    timers
}
