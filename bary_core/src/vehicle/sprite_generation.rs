use crate::prelude::*;
use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use image::{DynamicImage, RgbaImage};
use std::path::Path;

pub fn read_image(path: &Path) -> Option<RgbaImage> {
    Some(image::open(path).ok()?.to_rgba8())
}

pub fn diagram_color(part: &PartPrototype) -> Srgba {
    let cl = part.classification();
    match cl {
        PartClassification::Cargo => BLUE_500,
        PartClassification::Machine => RED_600,
        PartClassification::Thruster => ORANGE_300,
        PartClassification::Auxiliary => TEAL_700,
        PartClassification::DockingPort => GRAY_100,
        PartClassification::Other => GRAY_400,
    }
}

pub fn generate_image(vehicle: &Blueprint) -> Option<DynamicImage> {
    let (pixel_min, pixel_max) = vehicle.bounds();
    let dims = (pixel_max - pixel_min).inner().as_uvec2();
    let mut output = DynamicImage::new_rgba8(dims.x, dims.y);
    let to_export = output.as_mut_rgba8().unwrap();
    for layer in [PartLayer::Structural, PartLayer::Internal] {
        for (_, instance) in vehicle.parts() {
            if instance.prototype().layer() != layer {
                continue;
            }

            let dims = instance.dims_grid();

            let min = pixel_min.inner();
            let origin = instance.origin().inner();

            let px = (origin.x - min.x) as u32;
            let py = (origin.y - min.y) as u32;

            let pixels_lower = UVec2::new(px, py);
            let pixels_upper = pixels_lower + dims;

            let color: LinearRgba = diagram_color(&instance.prototype()).into();
            let color = color.to_f32_array();

            for x in pixels_lower.x..pixels_upper.x {
                for y in pixels_lower.y..pixels_upper.y {
                    let p = UVec2::new(x, y);

                    let Some(dst) =
                        to_export.get_pixel_mut_checked(p.x, to_export.height() - p.y - 1)
                    else {
                        continue;
                    };

                    for i in 0..3 {
                        dst.0[i] = (color[i] * 255.0).round() as u8;
                    }
                    dst.0[3] = 255;
                }
            }
        }
    }

    Some(output)
}
