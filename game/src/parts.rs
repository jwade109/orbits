use crate::planetary::GameState;
use crate::scenes::Render;
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
    }
}

#[derive(Component)]
pub struct StaticSprite(String, usize);

pub fn update_static_sprites(
    mut commands: Commands,
    assets: Res<AssetServer>,
    state: Res<GameState>,
    mut query: Query<(Entity, &mut Sprite, &mut Transform, &StaticSprite)>,
) {
    let sprites = GameState::sprites(&state);

    let mut sprite_entities: Vec<_> = query.iter_mut().collect();

    for (i, sprite) in sprites.iter().enumerate() {
        let pos = sprite.position.extend(sprite.z_index);
        let scale = Vec3::splat(sprite.scale);

        let found = sprite_entities
            .iter_mut()
            .find(|(_, _, _, s)| s.0 == sprite.path && s.1 == i);

        if let Some((_, _, tf, _)) = found {
            tf.translation = pos;
            tf.scale = scale;
        } else {
            let handle = assets.load_with_settings(
                sprite.path.clone(),
                |settings: &mut ImageLoaderSettings| {
                    settings.sampler = ImageSampler::nearest();
                },
            );
            let spr = Sprite::from_image(handle);
            let tf = Transform::from_scale(scale).with_translation(pos);
            commands.spawn((spr, tf, StaticSprite(sprite.path.clone(), i)));
        }
    }

    for (i, (e, _, _, _)) in query.iter().enumerate() {
        if i >= sprites.len() {
            commands.entity(e).despawn();
        }
    }
}
