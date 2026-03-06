use std::fs::File;
use std::io::Write;

use bary_core::prelude::*;
use bary_raylib::{
    systems::{TICKS_PER_SECOND, apparent_elapsed_time, find},
    world::update_world,
    world_builder::WorldBuilder,
};

struct SimEpoch {
    ticks: u64,
    x: f32,
    y: f32,
    angle: f32,
    target_x: f32,
    target_y: f32,
    target_angle: f32,
}

fn run_simulation(vehicle_name: &str, waypoint: Isometry2d, steps: usize, secs_per_step: f32) -> Vec<SimEpoch> {
    let mut world = WorldBuilder::new()
        .assets()
        .blueprint(vehicle_name)
        .spawn(vehicle_name, (0.0, 0.0, 0.0))
        .waypoint(vehicle_name, waypoint)
        .build();

    let grid_id = find::grid_by_name(&world.grids, vehicle_name).unwrap();

    let mut ret = Vec::new();

    for _ in 0..steps {
        let ticks = (secs_per_step * TICKS_PER_SECOND as f32).ceil() as u64;
        for _ in 0..ticks {
            update_world(&mut world);
        }

        let pose = find::grid_pose(&world.grids, grid_id).unwrap().to_tuple();

        let epoch = SimEpoch {
            ticks: world.ticks,
            x: pose.0,
            y: pose.1,
            angle: pose.2,
            target_x: waypoint.translation.x,
            target_y: waypoint.translation.y,
            target_angle: waypoint.rotation,
        };

        ret.push(epoch);
    }

    ret
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let args: Vec<_> = std::env::args().collect();
    // let outfile = args[1];

    let pos = randvec(3000.0, 12000.0);
    let waypoint = Isometry2d::new(pos, 0.0);

    let epochs = run_simulation("bellerophon", waypoint, 1000, 1.0);
    let mut file = File::create("sim.csv").unwrap();

    write!(file, "ticks,x,y,a,tx,ty,ta\n")?;

    for epoch in epochs {
        write!(
            file,
            "{},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3}\n",
            epoch.ticks,
            epoch.x,
            epoch.y,
            epoch.angle,
            epoch.target_x,
            epoch.target_y,
            epoch.target_angle,
        )?;
    }

    Ok(())
}
