use bary_core::prelude::*;
use bary_sim::Application;
use clap::Parser;
use log::info;

use bary_server::HeadlessServerApp;

/// Run the test client app
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(default_value = "5000")]
    server_port: u16,
    #[arg(default_value = "saves/")]
    savefile: Option<String>,
}

fn main() -> BaryResult<()> {
    let args = Args::parse();

    simple_logger::init_with_level(log::Level::Info).unwrap();

    info!("Starting dedicated server...");

    HeadlessServerApp::new(args.savefile, args.server_port, 10)?.spin_forever();

    Ok(())
}
