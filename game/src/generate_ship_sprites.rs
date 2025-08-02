use crate::scenes::SceneType;
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

pub fn make_terrain_sprite(chunk: &TerrainChunk) -> Image {
    let mut img = Image::new_fill(
        Extent3d {
            width: TILES_PER_CHUNK_SIDE as u32,
            height: TILES_PER_CHUNK_SIDE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &WHITE.with_alpha(0.0).to_u8_array(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();

    for (pos, tile) in chunk.tiles() {
        let pixel_pos = UVec2::new(pos.x, TILES_PER_CHUNK_SIDE as u32 - pos.y - 1);
        if let Some(pixel) = img.pixel_bytes_mut(pixel_pos.extend(0)) {
            let color = match tile {
                Tile::Air => continue,
                Tile::DeepStone => GRAY,
                Tile::Stone => LIGHT_GRAY,
                Tile::Sand => LIGHT_YELLOW,
                Tile::Ore => ORANGE,
                Tile::Grass => DARK_GREEN,
            };
            let c = color.to_u8_array();
            for i in 0..4 {
                pixel[i] = c[i];
            }
        }
    }

    img
}

pub fn proc_gen_ship_sprites(state: &mut GameState, images: &mut Assets<Image>) {
    for vehicle in state
        .universe
        .surface_vehicles
        .iter()
        .map(|(_, sv)| &sv.vehicle)
        .chain(
            state
                .universe
                .orbital_vehicles
                .iter()
                .map(|(_, ov)| &ov.vehicle),
        )
    {
        let sprite_name = vehicle_sprite_path(vehicle.discriminator());
        if state.image_handles.contains_key(&sprite_name) {
            continue;
        }

        let img = generate_ship_sprite(vehicle, &state.args.parts_dir(), false);
        if let Some(img) = img {
            println!(
                "Generated new ship sprite for {:016x} ({})",
                vehicle.discriminator(),
                vehicle.title(),
            );
            let dims = img.size();
            let handle = images.add(img);
            state.image_handles.insert(sprite_name, (handle, dims));
        }
    }
}

pub fn proc_gen_terrain_sprites(state: &mut GameState, images: &mut Assets<Image>) {
    if *state.current_scene().kind() != SceneType::Surface {
        return;
    }

    let surface_id = state.surface_context.current_surface;

    let ls = match state.universe.landing_sites.get(&surface_id) {
        Some(ls) => ls,
        None => return,
    };

    // spend no more than 2 ms doing this

    let start = std::time::Instant::now();
    let max_dur = std::time::Duration::from_millis(1);

    for (pos, chunk) in &ls.surface.terrain {
        let sprite_name = crate::scenes::surface::terrain_tile_sprite_name(surface_id, *pos);
        if state.image_handles.contains_key(&sprite_name) {
            continue;
        }
        let img = make_terrain_sprite(chunk);
        println!("Generated new terrain sprite {}", sprite_name);
        let dims = img.size();
        let handle = images.add(img);
        state.image_handles.insert(sprite_name, (handle, dims));

        let now = std::time::Instant::now();
        if now - start > max_dur {
            return;
        }
    }
}
