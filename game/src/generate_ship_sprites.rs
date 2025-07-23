use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image::RgbaImage;
use starling::prelude::*;
use std::path::Path;

use crate::drawing::vehicle_sprite_path;
use crate::game::GameState;

pub fn read_image(path: &Path) -> Option<RgbaImage> {
    Some(image::open(path).ok()?.to_rgba8())
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

pub fn proc_gen_ship_sprites(state: &mut GameState, images: &mut Assets<Image>) {
    for (_, (_, _, vehicle)) in &state.universe.surface_vehicles {
        let sprite_name = vehicle_sprite_path(vehicle.discriminator());
        if state.image_handles.contains_key(&sprite_name) {
            continue;
        }

        let img = generate_ship_sprite(vehicle, &state.args.parts_dir(), false);
        if let Some(img) = img {
            println!("Generated new ship sprite for {}", vehicle.discriminator());
            let dims = img.size();
            let handle = images.add(img);
            state.image_handles.insert(sprite_name, (handle, dims));
        }
    }
}
