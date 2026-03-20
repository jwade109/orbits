use std::fs::File;
use std::io::Write;

use bary_core::prelude::*;
use bary_raylib::{
    sim::systems::{TICKS_PER_SECOND, find, get_thruster_levels},
    sim::world::update_world,
    world_builder::WorldBuilder,
};

struct SimEpoch {
    ticks: u64,
    pose: Isometry2d,
    vel: Isometry2d,
    target: Isometry2d,
    acc_updates: u64,
    thrusters_firing: u32,
}

fn run_simulation(
    vehicle_name: &str,
    target: Isometry2d,
    steps: usize,
    secs_per_step: f32,
) -> Vec<SimEpoch> {
    let mut world = WorldBuilder::new()
        .assets()
        .blueprint(vehicle_name)
        .spawn(vehicle_name, (0.0, 0.0, 0.0))
        .waypoint(vehicle_name, target)
        .commands(vehicle_name)
        .build();

    let grid_id = find::grid_by_name(&world.grids, vehicle_name).unwrap();

    let mut ret = Vec::new();

    for _ in 0..steps {
        let ticks = (secs_per_step * TICKS_PER_SECOND as f32).ceil() as u64;
        for _ in 0..ticks {
            update_world(&mut world);
        }

        let pose = find::grid_pose(&world.grids, grid_id).unwrap();
        let vel = find::grid_vel(&world.grids, grid_id).unwrap();

        let thrusters = get_thruster_levels(grid_id, &world.grids, &world.thrusters).unwrap();

        let thrusters_firing = thrusters.into_iter().map(|e| e.1 as u32).sum();

        let epoch = SimEpoch {
            ticks: world.ticks,
            pose,
            vel,
            target,
            acc_updates: world.grid_acceleration_updates,
            thrusters_firing,
        };

        ret.push(epoch);
    }

    ret
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();

    let ship_name = args.get(1).unwrap();
    let x: f32 = args.get(2).unwrap().parse().unwrap();
    let y: f32 = args.get(3).unwrap().parse().unwrap();
    let r: f32 = args.get(4).unwrap().parse().unwrap();

    let waypoint = Isometry2d::new((x, y).into(), r);

    let epochs = run_simulation(ship_name, waypoint, 1000, 0.1);
    let mut file = File::create("sim.csv").unwrap();

    write!(file, "ticks,x,y,a,vx,vy,va,tx,ty,ta,updates,thrusters\n")?;

    for epoch in epochs {
        write!(
            file,
            "{},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{},{}\n",
            epoch.ticks,
            epoch.pose.translation.x,
            epoch.pose.translation.y,
            epoch.pose.rotation,
            epoch.vel.translation.x,
            epoch.vel.translation.y,
            epoch.vel.rotation,
            epoch.target.translation.x,
            epoch.target.translation.y,
            epoch.target.rotation,
            epoch.acc_updates,
            epoch.thrusters_firing,
        )?;
    }

    Ok(())
}
