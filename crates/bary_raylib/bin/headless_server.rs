use bary_core::prelude::*;
use bary_raylib::{headless_server::HeadlessServerApp, utils::Application};
use clap::Parser;
use log::info;

/// Run the test client app
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(default_value = "5000")]
    server_port: u16,
    #[arg(default_value = "saves/")]
    saves_dir: String,
    #[arg(default_value = "scenario_a")]
    save_name: String,
}

fn main() -> BaryResult<()> {
    let args = Args::parse();

    simple_logger::init_with_level(log::Level::Info).unwrap();

    info!("Starting dedicated server...");

    HeadlessServerApp::new(&args.saves_dir, args.server_port, 10)?.spin_forever();

    Ok(())
}
