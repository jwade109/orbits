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

    /// Multiplier to scale down by
    #[arg(long, short('x'), default_value = "1.0")]
    pub scale_factor: f32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    dbg!(&args);

    let parts = load_parts_from_dir(&args.parts_dir)?;

    let vehicle = load_vehicle(&args.ship_path, String::new(), &parts)?;

    let mut img =
        generate_image(&vehicle, &args.parts_dir, args.schematic).ok_or("Empty vehicle")?;

    if args.scale_factor < 1.0 {
        let filter = if args.schematic {
            image::imageops::FilterType::Nearest
        } else {
            image::imageops::FilterType::CatmullRom
        };
        img = img.resize(
            (img.width() as f32 * args.scale_factor).round() as u32,
            (img.height() as f32 * args.scale_factor).round() as u32,
            filter,
        );
    } else if args.scale_factor > 1.0 {
        img = img.resize(
            (img.width() as f32 * args.scale_factor).round() as u32,
            (img.height() as f32 * args.scale_factor).round() as u32,
            image::imageops::FilterType::Nearest,
        );
    }

    img.save(&args.out)?;

    Ok(())
}
