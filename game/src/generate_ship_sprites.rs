use crate::scenes::craft_editor::generate_image;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use starling::prelude::*;
use std::path::Path;

pub fn generate_ship_sprite(vehicle: &Vehicle, parts_dir: &Path) -> Option<Image> {
    println!("Generating sprite for vehicle {}", vehicle.name());
    let dynamic = generate_image(vehicle, parts_dir)?;
    let mut img = Image::from_dynamic(
        dynamic,
        true,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    Some(img)
}
