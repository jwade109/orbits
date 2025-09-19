use clap::Parser;
use starling::prelude::*;
use std::io::{LineWriter, Write};
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
    #[arg(long, short, default_value = "50")]
    pub x: f64,

    /// Target Y coordinate
    #[arg(long, short, default_value = "80")]
    pub y: f64,

    /// Target angle
    #[arg(long, short, default_value = "0.3")]
    pub angle: f64,
}

struct Simulation {
    pd: VehiclePd,
    convergance: Option<usize>,
}

fn simulate(
    vehicle: Vehicle,
    ticks: usize,
    target_pos: DVec2,
    target_angle: f64,
) -> Result<Simulation, Box<dyn std::error::Error>> {
    let mut sv = Spacecraft::from_vehicle(vehicle);
    sv.controller
        .set_policy(VehicleControlPolicy::hold_pos(target_pos, target_angle));
    let mut universe = Universe::empty();
    let id = universe.spawn_spacecraft(sv).ok_or("Expected ID")?;

    for i in 0..=ticks {
        let sv = universe.spacecraft.get(&id).ok_or("Failed to get SV")?;

        let converged = sv.body.pv.pos.distance(target_pos) < 10.0
            && sv.body.pv.vel.length() < 6.0
            && sv.body.angular_velocity.to_degrees().abs() < 4.0;

        // file.write_all(
        //     format!(
        //         "{},{},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{}\n",
        //         i,
        //         universe.stamp(),
        //         sv.body.pv.pos.x,
        //         sv.body.pv.pos.y,
        //         sv.body.pv.vel.x,
        //         sv.body.pv.vel.y,
        //         wrap_pi_npi_f64(args.angle - sv.body.angle),
        //         sv.body.angular_velocity,
        //         converged as u8,
        //     )
        //     .as_bytes(),
        // )?;
        universe.on_sim_tick(&ControlSignals::new(), false);

        if converged {
            break;
        }
    }

    todo!()
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

    for n in 0..args.sims {
        let vehicle = load_vehicle(&args.ship_path, String::new(), &parts)?;
        let data_path = args.outdir.join(format!("sim-{}.csv", n));
        let file = std::fs::File::create(&data_path)?;
        let mut file = LineWriter::new(file);
        println!("Writing to {}", data_path.display());
        file.write_all(b"tick,time,x,y,vx,vy,angular_error,angular_rate,converged\n")?;

        let mut sv = Spacecraft::from_vehicle(vehicle);
        sv.controller
            .set_policy(VehicleControlPolicy::hold_pos(target_pos, args.angle));
        let mut universe = Universe::empty();
        let id = universe.spawn_spacecraft(sv).ok_or("Expected ID")?;

        for i in 0..=args.ticks {
            let sv = universe.spacecraft.get(&id).ok_or("Failed to get SV")?;

            let converged = sv.body.pv.pos.distance(target_pos) < 10.0
                && sv.body.pv.vel.length() < 6.0
                && sv.body.angular_velocity.to_degrees().abs() < 4.0;

            file.write_all(
                format!(
                    "{},{},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{}\n",
                    i,
                    universe.stamp(),
                    sv.body.pv.pos.x,
                    sv.body.pv.pos.y,
                    sv.body.pv.vel.x,
                    sv.body.pv.vel.y,
                    wrap_pi_npi_f64(args.angle - sv.body.angle),
                    sv.body.angular_velocity,
                    converged as u8,
                )
                .as_bytes(),
            )?;
            universe.on_sim_tick(&ControlSignals::new(), false);

            if converged {
                break;
            }
        }
    }

    println!("Done.");

    Ok(())
}
