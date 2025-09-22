use clap::Parser;
use serde::{Deserialize, Serialize};
use starling::prelude::*;
use std::path::PathBuf;

/// Simulates a vehicle operating under a particular command
#[derive(Parser, Debug, Default, Clone)]
struct Args {
    /// Ship file (.vehicle) location
    #[arg(long, short('s'))]
    pub ship_path: PathBuf,

    /// Folder containing part definitions
    #[arg(long, short)]
    pub parts_dir: PathBuf,

    /// Result destination filepath
    #[arg(long, short)]
    pub outdir: PathBuf,

    /// Number of ticks to simulate for
    #[arg(long, short, default_value = "10000")]
    pub ticks: usize,

    /// Number of simulations to run
    #[arg(long, short, default_value = "10")]
    pub sims: usize,

    /// Target X coordinate
    #[arg(long, short, default_value = "50", allow_hyphen_values = true)]
    pub x: f64,

    /// Target Y coordinate
    #[arg(long, short, default_value = "80", allow_hyphen_values = true)]
    pub y: f64,

    /// Target angle
    #[arg(long, short, default_value = "0.3", allow_hyphen_values = true)]
    pub angle: f64,

    /// Rate of exploration through PD coefficient space
    #[arg(long, short, default_value = "0.2")]
    pub exploration_rate: f32,
}

#[derive(Debug, Deserialize, Serialize)]
struct Simulation {
    pd: VehiclePd,
    convergence: Option<Nanotime>,
    target_pos: DVec2,
    target_angle: Vec<f64>,
    t: Vec<Nanotime>,
    x: Vec<f64>,
    y: Vec<f64>,
    a: Vec<f64>,
    accel: Vec<f64>,
}

impl Simulation {
    pub fn new(pd: VehiclePd, target_pos: DVec2) -> Self {
        Self {
            pd,
            convergence: None,
            target_pos,
            target_angle: Vec::new(),
            t: Vec::new(),
            x: Vec::new(),
            y: Vec::new(),
            a: Vec::new(),
            accel: Vec::new(),
        }
    }
}

fn simulate(
    vehicle: Vehicle,
    ticks: usize,
    target_pos: DVec2,
    target_angle: f64,
) -> Result<Simulation, Box<dyn std::error::Error>> {
    let mut sim = Simulation::new(vehicle.pid, target_pos);

    let sv = Spacecraft::from_vehicle(vehicle);
    let mut universe = Universe::empty();
    let id = universe.spawn_spacecraft(sv).ok_or("Expected ID")?;

    let mut sum_accel = 0.0;

    for i in 0..=ticks {
        let t = universe.stamp();
        let sv = universe.spacecraft.get_mut(&id).ok_or("Failed to get SV")?;
        sv.controller
            .set_policy(VehicleControlPolicy::hold_pos(target_pos, target_angle));

        let converged = sv.body.pv.pos.distance(target_pos) < 20.0 && sv.body.pv.vel.length() < 4.0;
        // && sv.body.angular_velocity.to_degrees().abs() < 5.0
        // && wrap_pi_npi_f64(sv.body.angle - target_angle)
        //     .abs()
        //     .to_degrees()
        //     < 5.0;

        let accel = sv.vehicle.body_frame_accel().linear.length();
        sum_accel += accel;

        if i % 10 == 0 {
            sim.t.push(t);
            sim.x.push(sv.body.pv.pos.x);
            sim.y.push(sv.body.pv.pos.y);
            sim.a.push(sv.body.angle);
            sim.accel.push(sum_accel);
        }

        universe.on_sim_tick(&ControlSignals::new(), false);

        if converged {
            sim.convergence = Some(universe.stamp());
            break;
        }
    }

    Ok(sim)
}

struct TuningResult {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    dbg!(&args);

    let parts = load_parts_from_dir(&args.parts_dir)?;

    let target_pos = DVec2::new(args.x, args.y);

    if std::fs::exists(&args.outdir)? {
        std::fs::remove_dir_all(&args.outdir)?;
    }
    std::fs::create_dir(&args.outdir)?;

    let vehicle = load_vehicle(&args.ship_path, String::new(), &parts)?;

    for n in 0..args.sims {
        let mut vehicle = vehicle.clone();
        if n > 0 {
            vehicle.pid.attitude_controller = vehicle
                .pid
                .attitude_controller
                .jitter(args.exploration_rate + 1.0);
        }
        let sim = simulate(vehicle.clone(), args.ticks, target_pos, args.angle)?;
        // vehicle.pid.docking_linear_controller = vehicle.pid.docking_linear_controller.jitter();
        // vehicle.pid.horizontal_controller = vehicle.pid.horizontal_controller.jitter();
        // vehicle.pid.vertical_controller = vehicle.pid.vertical_controller.jitter();
        let data_path = args.outdir.join(format!("sim-{}.yaml", n));
        let str = serde_yaml::to_string(&sim)?;
        std::fs::write(data_path, str)?;
        if sim.convergence.is_some() {
            dbg!(sim.pd);
        }
    }

    println!("Done.");

    Ok(())
}
