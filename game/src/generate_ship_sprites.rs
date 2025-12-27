use crate::starling::prelude::*;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use image::RgbaImage;
use std::path::Path;

pub fn read_image(path: &Path) -> Option<RgbaImage> {
    Some(image::open(path).ok()?.to_rgba8())
}

pub fn generate_ship_sprite(vehicle: &Blueprint, parts_dir: &Path, schematic: bool) -> Option<Image> {
    let dynamic = generate_image(vehicle, parts_dir, schematic)?;
    let mut img = Image::from_dynamic(
        dynamic,
        true,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    Some(img)
}
