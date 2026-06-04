use crate::PingParticle;
use crate::*;
use bary_core::prelude::*;
use bary_orbital::*;
use std::collections::BTreeSet;

/// Updates ring particles.
pub fn sys_update_ring_particles(particles: &mut Vec<PingParticle>, current_tick: u64) {
    particles.retain(|p| p.is_alive(current_tick));
}

/// Updates vehicle rigid body physics.
pub fn sys_propagate_grid_rigid_bodies(grids: &mut Components<VehicleGrid>) {
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
pub fn sys_update_thrusters(
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

/// Appends grid locations to tracker entities.
pub fn sys_update_trackers(
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
        tracker.add(
            grid.origin(),
            grid.particle_location,
            grid.centroid_isometry(),
        );
    }

    for id in to_despawn {
        _ = trackers.despawn(id);
    }
}

pub fn get_excavator_tiles(
    id: Ent,
    ex: &Excavator,
    world: &World,
) -> BaryResult<Option<(Ent, Vec<GlobalTileIndex>)>> {
    let part = world.parts.try_get(id)?;
    let grid = world.grids.try_get(part.grid_id)?;
    let part_iso = grid.origin() * part.region.center_isometry();

    // TODO(gross) spatial lookups here or something.
    for (rock_id, rock) in world.asteroids.iter() {
        let wrt_asteroid = in_frame(rock.iso, part_iso.translation);

        if wrt_asteroid.length() > rock.ast.max_radius() {
            continue;
        }

        let mut tiles = Vec::new();

        let gc = GlobalTileIndex(vfloor(wrt_asteroid / TERRAIN_TILE_WIDTH_METERS));

        let ri = 2 * (ex.radius / TERRAIN_TILE_WIDTH_METERS).ceil() as i32;

        for x in -ri..ri {
            for y in -ri..ri {
                let offset = IVec2::new(x, y);
                let g = GlobalTileIndex(gc.0 + offset);
                let c = g.center_isometry();
                let dist = c.translation.distance(wrt_asteroid);
                if dist < ex.radius {
                    tiles.push(g);
                    //     let o = g.origin_isometry();
                    //     fill_rectangle(d, ast.iso * o, tile_dims, Color::TEAL.alpha(0.5));
                }
            }
        }

        return Ok(Some((*rock_id, tiles)));
    }

    Ok(None)
}

/// Updates computers according to their current
/// directive and the thruster state of the parent vehicle.
pub fn sys_update_computers(
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
