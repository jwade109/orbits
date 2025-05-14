use bevy::asset::embedded_asset;
use bevy::image::{ImageLoaderSettings, ImageSampler};
use bevy::prelude::*;

pub struct PartPlugin;

impl Plugin for PartPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "src/", "../assets/parts/frame.png");
        embedded_asset!(app, "src/", "../assets/parts/tank11.png");
        embedded_asset!(app, "src/", "../assets/parts/tank21.png");
        embedded_asset!(app, "src/", "../assets/parts/tank22.png");
        embedded_asset!(app, "src/", "../assets/parts/motor.png");
        embedded_asset!(app, "src/", "../assets/parts/antenna.png");

        app.add_systems(Startup, add_part_sprites);
    }
}

fn add_part_sprites(mut commands: Commands, assets: Res<AssetServer>, images: Res<Assets<Image>>) {
    for (i, part) in ["frame", "tank11", "tank21", "tank22", "motor", "antenna"]
        .into_iter()
        .enumerate()
    {
        let path = format!("../assets/parts/{}.png", part);
        let handle = assets.load_with_settings(path, |settings: &mut ImageLoaderSettings| {
            // Need to use nearest filtering to avoid bleeding between the slices with tiling
            settings.sampler = ImageSampler::nearest();
        });

        let d = if let Some(image) = images.get(&handle) {
            image.width().max(image.height())
        } else {
            10
        };

        let sprite = Sprite::from_image(handle);

        let scale = 100.0 / d as f32;

        let tf = Transform::from_scale(Vec3::splat(scale))
            .with_translation(Vec3::X * (i as f32 * 200.0 - 400.0));
        commands.spawn((sprite, tf));
    }
}
