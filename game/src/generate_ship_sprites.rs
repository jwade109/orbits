use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
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

pub fn proc_gen_ship_sprites(state: &mut GameState, images: &mut Assets<Image>) {
    for vehicle in state
        .universe
        .spacecraft
        .iter()
        .map(|(_, sv)| &sv.vehicle)
        .chain(state.universe.spacecraft.iter().map(|(_, ov)| ov.vehicle()))
    {
        let sprite_name = vehicle_sprite_path(vehicle.discriminator());
        if state.image_handles.contains_key(&sprite_name) {
            continue;
        }

        let img = generate_ship_sprite(vehicle, &state.args.parts_dir(), false);
        if let Some(img) = img {
            println!(
                "Generated new ship sprite for {:0x} ({})",
                vehicle.discriminator(),
                vehicle.title(),
            );
            let dims = img.size();
            let handle = images.add(img);
            state.image_handles.insert(sprite_name, (handle, dims));
        }
    }

    for (_, ast) in &mut state.universe.asteroids {
        use bevy::image::*;

        let mut gen = Vec::new();

        for zone in ast.zones() {
            let sprite_name = ast.sprite_name(zone);
            let is_changed = ast.is_changed(zone);
            if state.image_handles.contains_key(&sprite_name) && !is_changed {
                continue;
            }
            let viewport = zone.aabb();
            let img = make_asteroid_image(ast, viewport, 60, None, false, false);
            let img = image::DynamicImage::from(img);
            let mut img = Image::from_dynamic(
                img,
                true,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            img.sampler = bevy::image::ImageSampler::nearest();
            let dims = img.size();
            let handle = images.add(img);
            println!("Generated new sprite for asteroid \"{}\"", sprite_name);
            state.image_handles.insert(sprite_name, (handle, dims));
            gen.push(zone);
        }

        for z in gen {
            ast.clear_changed(z);
        }
    }
}
