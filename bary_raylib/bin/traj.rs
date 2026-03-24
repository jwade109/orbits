use std::fs::File;
use std::io::Write;

use bary_core::prelude::*;
use bary_raylib::{
    ops,
    query::primary_computer_id,
    sim::{
        TimedInstruction,
        systems::{TICKS_PER_SECOND, find, get_thruster_levels},
        world::update_world,
    },
    world_builder::WorldBuilder,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimSpec {
    ship_name: String,
    instructions: Vec<TimedInstruction>,
    steps: u64,
    secs_per_step: f32,
}

fn load_sim_file(filename: &str) -> Result<SimSpec, Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(filename)?;
    let spec: SimSpec = serde_yaml::from_str(&s)?;
    Ok(spec)
}

struct SimEpoch {
    ticks: u64,
    pose: Isometry2d,
    vel: Isometry2d,
    target: Option<Isometry2d>,
    acc_updates: u64,
    thrusters_firing: u32,
}

fn run_simulation(
    vehicle_name: &str,
    instructions: Vec<TimedInstruction>,
    steps: u64,
    secs_per_step: f32,
) -> Vec<SimEpoch> {
    let mut world = WorldBuilder::new()
        .assets()
        .blueprint(vehicle_name)
        .spawn(vehicle_name, (0.0, 0.0, 0.0))
        .build();

    let grid_id = find::grid_by_name(&world.grids, vehicle_name).unwrap();

    _ = ops::set_primary_computer_state(grid_id, true, &mut world);

    let cpu_id = find::primary_computer_id(grid_id, &world.grids).unwrap();

    let cpu = world.computers.try_get_mut(cpu_id).unwrap();

    cpu.command_queue = instructions;

    let mut ret = Vec::new();

    for _ in 0..steps {
        let ticks = (secs_per_step * TICKS_PER_SECOND as f32).ceil() as u64;
        for _ in 0..ticks {
            update_world(&mut world);
        }

        let cpu_id = primary_computer_id(grid_id, &world.grids).unwrap();
        let cpu = world.computers.try_get(cpu_id).unwrap();
        let pose = find::grid_pose(&world.grids, grid_id).unwrap();
        let vel = find::grid_vel(&world.grids, grid_id).unwrap();

        let thrusters = get_thruster_levels(grid_id, &world.grids, &world.thrusters).unwrap();

        let thrusters_firing = thrusters.into_iter().map(|e| e.1 as u32).sum();

        let epoch = SimEpoch {
            ticks: world.ticks,
            pose,
            vel,
            target: cpu.current_waypoint(),
            acc_updates: world.grid_acceleration_updates,
            thrusters_firing,
        };

        ret.push(epoch);
    }

    ret
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();

    let filename = args.get(1).unwrap();

    let spec = load_sim_file(filename).unwrap();

    let epochs = run_simulation(
        &spec.ship_name,
        spec.instructions.clone(),
        spec.steps,
        spec.secs_per_step,
    );
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
            epoch
                .target
                .map(|t| t.translation.x)
                .unwrap_or(std::f32::NAN),
            epoch
                .target
                .map(|t| t.translation.y)
                .unwrap_or(std::f32::NAN),
            epoch.target.map(|t| t.rotation).unwrap_or(std::f32::NAN),
            epoch.acc_updates,
            epoch.thrusters_firing,
        )?;
    }

    Ok(())
}
