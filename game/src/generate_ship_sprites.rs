use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image::{DynamicImage, RgbaImage};
use starling::prelude::*;
use std::path::Path;

pub fn read_image(path: &Path) -> Option<RgbaImage> {
    Some(image::open(path).ok()?.to_rgba8())
}

pub fn diagram_color(part: &PartPrototype) -> Srgba {
    match part {
        PartPrototype::Cargo(..) => GREEN,
        PartPrototype::Thruster(..) => RED,
        PartPrototype::Tank(..) => ORANGE,
        _ => match part.layer() {
            PartLayer::Exterior => DARK_GRAY,
            PartLayer::Internal => GRAY,
            PartLayer::Structural => WHITE,
            PartLayer::Plumbing => PURPLE,
        },
    }
}

pub fn generate_ship_sprite(vehicle: &Vehicle, parts_dir: &Path, schematic: bool) -> Option<Image> {
    let dynamic = generate_image(vehicle, parts_dir, schematic)?;
    let mut img = Image::from_dynamic(
        dynamic,
        true,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    Some(img)
}

pub fn generate_error_sprite() -> Image {
    Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &WHITE.to_u8_array(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}
