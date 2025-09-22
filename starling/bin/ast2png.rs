use clap::Parser;
use starling::prelude::*;
use std::path::PathBuf;

/// Converts ship file to PNG
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about)]
pub struct Args {
    /// Asteroid seed
    #[arg(long, short)]
    seed: Option<u64>,

    /// Render image with this width
    #[arg(long, short)]
    width: u32,

    /// Resize resultant image to this size
    #[arg(long, default_value = "400")]
    resize_to: u32,

    /// Resize resultant image to this size
    #[arg(long, short, default_value = "400")]
    radius: f32,

    /// Apply light cast at this angle
    #[arg(long, short)]
    light_angle: Option<f32>,

    /// Draw debug info
    #[arg(long, short)]
    debug: bool,

    /// Highlight ore deposits
    #[arg(long, short)]
    highlight_deposits: bool,

    /// X chunk coordinate
    #[arg(short, allow_hyphen_values = true)]
    x: Option<i32>,

    /// Y chunk coordinate
    #[arg(short, allow_hyphen_values = true)]
    y: Option<i32>,

    /// Focus on particular point on the surface
    #[arg(short('a'), allow_hyphen_values = true)]
    surface_angle: Option<f32>,

    /// Output file path
    #[arg(long, short)]
    outpath: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    dbg!(&args);

    let seed = args.seed.unwrap_or(randint(1000, 1000000) as u64);

    println!("Seed: {}", seed);

    let ast = Asteroid::random(args.radius, seed);

    let viewport = if let Some((x, y)) = args.x.zip(args.y) {
        let center = Vec2::new(x as f32 * 100.0, y as f32 * 100.0);
        let span = Vec2::splat(100.0);
        AABB::new(center, span)
    } else if let Some(a) = args.surface_angle {
        let radius = ast.radius_at(a);
        let p = rotate(Vec2::X * radius, a);
        AABB::new(p, Vec2::splat(100.0))
    } else {
        AABB::new(Vec2::ZERO, Vec2::splat(ast.max_radius() * 2.0))
    };

    let img = make_asteroid_image(
        &ast,
        viewport,
        args.width,
        args.light_angle,
        args.debug,
        args.highlight_deposits,
    );

    let img = image::DynamicImage::from(img);
    let img = img.resize(
        args.resize_to,
        args.resize_to,
        image::imageops::FilterType::Nearest,
    );

    Ok(img.save(args.outpath)?)
}
