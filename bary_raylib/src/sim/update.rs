use crate::client::*;
use crate::constants::*;
use crate::sim::*;
use crate::utils::DebugTimers;
use bary_core::prelude::*;
use early_returns::*;
use std::collections::*;

fn sys_propagate_grid_rigid_bodies(grids: &mut Components<VehicleGrid>) {
    for grid in grids.values_mut() {
        let body_frame_accel = grid.linear_acceleration();
        let omega = grid.angular_acceleration();
        let accel = rotate(body_frame_accel, grid.particle_location.rotation);
        grid.particle_location.translation += grid.velocity.translation * NOMINAL_DT;
        grid.velocity.translation += accel * NOMINAL_DT;
        grid.particle_location.rotation += grid.velocity.rotation * NOMINAL_DT;
        grid.velocity.rotation += omega * NOMINAL_DT;
    }
}

fn sys_update_thrusters(
    thrusters: &mut Components<Thruster>,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    computers: &Components<Computer>,
) -> BTreeSet<Ent> {
    let mut needs_update = BTreeSet::new();

    for (grid_id, grid) in grids.iter() {
        if grid.thrusters.is_empty() {
            continue;
        }

        let Some(cpu_id) = grid.computers.first() else {
            continue;
        };
        let Ok(cpu) = computers.try_get(*cpu_id) else {
            continue;
        };
        if !cpu.fired_this_tick {
            continue;
        }

        let mut thruster_changed = false;

        for thruster_id in &grid.thrusters {
            let Ok(thruster) = thrusters.try_get_mut(*thruster_id) else {
                continue;
            };
            let Ok(part) = parts.try_get(*thruster_id) else {
                continue;
            };

            let ctrl = cpu.vehicle_control;

            let tac = match part.region.rot() {
                Rotation::East => ctrl.plus_x,
                Rotation::North => ctrl.neg_y,
                Rotation::West => ctrl.neg_x,
                Rotation::South => ctrl.plus_y,
            };

            // TODO(optimization) reduce lookups by storing isometry on the thruster?
            let isometry = part.region.center_isometry();
            let center_of_thrust = isometry.translation;
            let rotation = part.region.rot();
            let wrench = body_frame_wrench(
                thruster.thrust,
                center_of_thrust,
                rotation,
                grid.center_of_mass,
            );

            let old_val = thruster.is_on;

            if thruster.is_rcs {
                let can_torque = wrench.rotation.abs() > 0.5 && ctrl.attitude.abs() > 0.5;
                let is_torque =
                    can_torque && wrench.rotation.signum() as f64 == ctrl.attitude.signum();
                let is_linear = tac.throttle > 0.0 && tac.use_rcs;
                thruster.is_on = is_linear || is_torque;
            } else {
                thruster.is_on = !tac.use_rcs && tac.throttle > 0.0;
            }

            thruster_changed |= old_val != thruster.is_on;

            thruster.last_controlled_by = Some(*cpu_id);
        }

        if thruster_changed {
            needs_update.insert(*grid_id);
        }
    }

    needs_update
}

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

fn sys_update_trackers(
    trackers: &mut Components<Tracker>,
    grids: &Components<VehicleGrid>,
    ticks: u64,
) {
    if ticks % 20 > 0 {
        return;
    }

    let mut to_despawn = BTreeSet::new();

    for (grid_id, grid) in grids.iter() {
        let is_controllable = !grid.computers.is_empty();
        if trackers.try_get(*grid_id).is_err() && is_controllable {
            let tracker = Tracker::default();
            trackers.spawn(*grid_id, tracker);
        }
    }

    for (grid_id, tracker) in trackers.iter_mut() {
        let Ok(grid) = grids.try_get(*grid_id) else {
            to_despawn.insert(*grid_id);
            continue;
        };
        tracker.add(grid);
    }

    for id in to_despawn {
        _ = trackers.despawn(id);
    }
}

fn sys_update_machines(world: &mut World) {
    for (part_id, machine) in world.machines.iter_mut() {
        step_process(machine, *part_id, &mut world.inventories);
    }
}

fn sys_update_ring_particles(particles: &mut Vec<PingParticle>) {
    for ring in particles.iter_mut() {
        ring.step()
    }
    particles.retain(|p| p.is_alive());
}

pub fn update_world(world: &mut World) -> DebugTimers {
    world.ticks += 1;

    let mut timers = DebugTimers::default();
    timers.ticks += 1;

    {
        let _timer = timers.scope("grid_motion");

        sys_update_ring_particles(&mut world.particles);
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
        let _timer = timers.scope("disabled_update_pipes");

        // sys_update_pipes(&mut world.inventories, &mut world.pipes);
    }

    {
        let _timer = timers.scope("old_fill_inventories");

        sys_fill_inventories_attached_to_debug_sources(world);
    }

    {
        let _timer = timers.scope("new_grid_inventories");

        sys_insert_gridventories(
            &world.grids,
            &mut world.gridventories,
            &world.blueprints,
            &world.prototypes,
        );

        sys_update_gridventories(&mut world.gridventories);
    }

    {
        let _timer = timers.scope("update_machines");

        sys_update_machines(world);
    }

    timers
}

fn sys_insert_gridventories(
    grids: &Components<VehicleGrid>,
    gv: &mut Components<GridVentory>,
    bp: &Components<NamedBlueprint>,
    protos: &Components<PartPrototype>,
) {
    for (grid_id, grid) in grids.iter() {
        if let Some(id) = &grid.blueprint {
            if let Some(blueprint) = blueprint_by_id(bp, id) {
                gv.entry(*grid_id)
                    .or_insert_with(|| GridVentory::from_blueprint(blueprint, protos));
            }
        }
    }
}

fn sys_update_gridventories(gv: &mut Components<GridVentory>) {
    for gv in gv.values_mut() {
        update_inventory(gv);
    }
}
