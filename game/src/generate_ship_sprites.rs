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

pub fn generate_image(
    vehicle: &Vehicle,
    parts_dir: &Path,
    schematic: bool,
) -> Option<DynamicImage> {
    let (pixel_min, pixel_max) = vehicle.pixel_bounds()?;
    let dims = pixel_max - pixel_min;
    let mut img = DynamicImage::new_rgba8(dims.x as u32, dims.y as u32);
    let to_export = img.as_mut_rgba8().unwrap();
    for (pos, rot, part) in vehicle.parts_by_layer() {
        let path = parts_dir.join(&part.path).join("skin.png");
        let img = match read_image(&path) {
            Some(img) => img,
            None => {
                println!("Failed to read {}", path.display());
                continue;
            }
        };

        let px = (pos.x - pixel_min.x) as u32;
        let py = (pos.y - pixel_min.y) as u32;

        let color = match part.data.class {
            PartClass::Cargo => GREEN,
            PartClass::Thruster(_) => RED,
            PartClass::Tank(_) => ORANGE,
            _ => match part.data.layer {
                PartLayer::Exterior => continue,
                PartLayer::Internal => GRAY,
                PartLayer::Structural => WHITE,
            },
        }
        .mix(&BLACK, 0.3)
        .to_f32_array();

        for x in 0..img.width() {
            for y in 0..img.height() {
                let p = IVec2::new(x as i32, y as i32);
                let xp = img.width() as i32 - p.x - 1;
                let yp = img.height() as i32 - p.y - 1;
                let p = match *rot {
                    Rotation::East => IVec2::new(p.x, yp),
                    Rotation::North => IVec2::new(p.y, p.x),
                    Rotation::West => IVec2::new(xp, p.y),
                    Rotation::South => IVec2::new(yp, xp),
                }
                .as_uvec2();

                let src = img.get_pixel_checked(x, y);
                let dst =
                    to_export.get_pixel_mut_checked(px + p.x, to_export.height() - (py + p.y) - 1);
                if let Some((src, dst)) = src.zip(dst) {
                    if src.0[3] > 0 {
                        for i in 0..3 {
                            dst.0[i] = if schematic {
                                (color[i] * 255.0) as u8
                            } else {
                                src.0[i]
                            };
                        }
                        dst.0[3] = 255;
                    }
                }
            }
        }
    }

    Some(img)
}

pub fn generate_ship_sprite(vehicle: &Vehicle, parts_dir: &Path, schematic: bool) -> Option<Image> {
    println!("Generating sprite for vehicle {}", vehicle.name());
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
