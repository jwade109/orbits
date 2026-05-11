use crate::client::*;
use crate::constants::*;
use crate::sim::*;
use bary_core::prelude::*;
use bary_factory::*;
use bary_orbital::attitude_control_law;
use bary_orbital::position_hold_control_law;
use bary_parts::*;
use early_returns::*;
use std::collections::*;

/// Updates vehicle rigid body physics.
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

/// Updates vehicle thruster states according to the
/// primary computer on the parent grid. If the grid
/// has no computer, or that computer has not changed
/// its command, thruster states will not change.
/// Returns a list of grids which have changed their
/// thruster states.
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

/// Appends grid locations to tracker entities.
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

/// Steps running machines forward by one tick, and modifies their
/// corresponding inventory if necessary.
fn sys_update_machines(world: &mut World) {
    for (part_id, machine) in world.machines.iter_mut() {
        step_process(machine, *part_id, &mut world.inventories);
    }
}

/// Updates ring particles.
fn sys_update_ring_particles(particles: &mut Vec<PingParticle>) {
    for ring in particles.iter_mut() {
        ring.step()
    }
    particles.retain(|p| p.is_alive());
}

/// Updates computers according to their current
/// directive and the thruster state of the parent vehicle.
fn sys_update_computers(
    computers: &mut Components<Computer>,
    parts: &Components<Part>,
    grids: &Components<VehicleGrid>,
) {
    for (cpu_id, computer) in computers.iter_mut() {
        computer.tick_forward();

        if !computer.fired_this_tick {
            continue;
        }

        if let Some(ctrl) = computer.current_control() {
            computer.vehicle_control = ctrl;
        } else if let Some(target_pose) = computer.current_waypoint() {
            let Ok(part) = parts.try_get(*cpu_id) else {
                continue;
            };

            let Ok(grid) = grids.try_get(part.grid_id) else {
                continue;
            };

            let pose = grid.particle_location;

            let target = PV::from_f64(target_pose.translation, Vec2::ZERO);
            let actual = PV::from_f64(pose.translation, grid.velocity.translation);

            let body = RigidBody {
                pv: actual,
                angle: pose.rotation as f64,
                angular_velocity: grid.velocity.rotation as f64,
            };

            let (ctrl, _status) =
                position_hold_control_law(target, target_pose.rotation as f64, &body, DVec2::ZERO);

            computer.vehicle_control = ctrl;
        } else if let Some(target) = computer.current_angle() {
            let Ok(part) = parts.try_get(*cpu_id) else {
                continue;
            };

            let Ok(grid) = grids.try_get(part.grid_id) else {
                continue;
            };

            let actual = Angle::radians(grid.particle_location.rotation);

            let body = RigidBody {
                pv: PV::ZERO,
                angle: actual.as_rad() as f64,
                angular_velocity: grid.velocity.rotation as f64,
            };

            let pid = PDCtrl::new(20.0, 50.0);

            let (ctrl, _status) = attitude_control_law(target.as_rad() as f64, &pid, &body);

            computer.vehicle_control = ctrl;
        } else if let Some(dv) = computer.delta_v_mut() {
            *dv -= Vec2::splat(0.1);
            let Ok(part) = parts.try_get(*cpu_id) else {
                continue;
            };

            let Ok(grid) = grids.try_get(part.grid_id) else {
                continue;
            };
            let target = dv.to_angle();
            let pid = PDCtrl::new(20.0, 50.0);
            let body = RigidBody {
                pv: PV::ZERO,
                angle: grid.particle_location.rotation as f64,
                angular_velocity: grid.velocity.rotation as f64,
            };
            let (ctrl, _status) = attitude_control_law(target as f64, &pid, &body);

            computer.vehicle_control = ctrl;
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

    timers
}
