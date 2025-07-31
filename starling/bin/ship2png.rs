use clap::Parser;
use starling::prelude::*;
use std::path::PathBuf;

/// Converts ship file to PNG
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Ship file (.vehicle) location
    #[arg(long, short('s'))]
    pub ship_path: PathBuf,

    /// Folder containing part definitions
    #[arg(long, short)]
    pub parts_dir: PathBuf,

    /// Destination filepath for PNG
    #[arg(long, short)]
    pub out: PathBuf,

    /// Whether to draw as schematic or "realistic"
    #[arg(long, short('c'))]
    pub schematic: bool,

    /// Multiplier to scale up by
    #[arg(long, short('g'), default_value = "10")]
    pub grow: i8,

    /// Multiplier to scale down by
    #[arg(long, short('x'), default_value = "0")]
    pub scale: f32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    dbg!(&args);

    let parts = load_parts_from_dir(&args.parts_dir)?;

    let vehicle = load_vehicle(&args.ship_path, String::new(), &parts)?;

    let mut img =
        generate_image(&vehicle, &args.parts_dir, args.schematic).ok_or("Empty vehicle")?;

    if args.scale < 1.0 {
        let filter = if args.schematic {
            image::imageops::FilterType::Nearest
        } else {
            image::imageops::FilterType::CatmullRom
        };
        img = img.resize(
            (img.width() as f32 * args.scale).round() as u32,
            (img.height() as f32 * args.scale).round() as u32,
            filter,
        );
    }

    // scale up so viewers don't blur pixels
    img = img.resize(
        img.width() * 10,
        img.height() * 10,
        image::imageops::FilterType::Nearest,
    );

    img.save(&args.out)?;

    Ok(())
}
