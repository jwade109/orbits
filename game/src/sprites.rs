use crate::game::GameState;
use crate::scenes::CameraProjection;
use crate::scenes::*;
use bevy::asset::embedded_asset;
use bevy::prelude::*;
use starling::prelude::*;

pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "src/", "../assets/Earth.png");
        embedded_asset!(app, "src/", "../assets/Luna.png");
        embedded_asset!(app, "src/", "../assets/Asteroid.png");
        embedded_asset!(app, "src/", "../assets/shadow.png");
        embedded_asset!(app, "src/", "../assets/spacecraft.png");
        embedded_asset!(app, "src/", "../assets/collision_pixel.png");
    }
}

// const SELECTED_SPACECRAFT_Z_INDEX: f32 = 8.0;
const SHADOW_Z_INDEX: f32 = 7.0;

const EXPECTED_SHADOW_SPRITE_HEIGHT: u32 = 1000;

#[derive(Component)]
#[require(Transform)]
pub struct ShadowTexture(PlanetId);

pub fn update_shadow_sprites(
    mut commands: Commands,
    mut query: Query<(Entity, &ShadowTexture, &mut Transform, &mut Visibility)>,
    state: Res<GameState>,
) {
    let scene = state.current_scene();

    match scene.kind() {
        SceneType::Orbital => (),
        _ => {
            for (e, _, _, _) in query.iter() {
                commands.entity(e).despawn();
            }
            return;
        }
    }

    for (e, ShadowTexture(id), mut transform, mut vis) in query.iter_mut() {
        let lup = match state.lup_planet(*id, state.sim_time) {
            Some(lup) => lup,
            None => {
                commands.entity(e).despawn();
                println!("Despawn shadow for {}", id);
                continue;
            }
        };

        *vis = match state.orbital_context.draw_mode {
            DrawMode::Default => Visibility::Visible,
            _ => Visibility::Hidden,
        };

        let body = match lup.body() {
            Some(b) => b,
            None => {
                commands.entity(e).despawn();
                continue;
            }
        };

        let angle = PI - state.light_source().angle_to(Vec2::X);
        let scale = (2.0 * body.radius) / EXPECTED_SHADOW_SPRITE_HEIGHT as f32
            * state.orbital_context.scale();
        let pos = lup.pv().pos_f32();
        transform.translation = state.orbital_context.w2c(pos).extend(SHADOW_Z_INDEX);
        transform.scale = Vec3::new(scale, scale, 1.0);
        transform.rotation = Quat::from_rotation_z(angle);
    }
}

pub fn hashable_to_color(h: &impl std::hash::Hash) -> Hsla {
    use std::hash::Hasher;
    let mut s = std::hash::DefaultHasher::new();
    h.hash(&mut s);
    let h: u64 = s.finish() % 1000;
    let hue = 360.0 * (h as f32 / 1000 as f32);
    Hsla::new(hue, 1.0, 0.5, 1.0)
}

pub fn update_background_sprite(
    mut camera: Single<&mut Camera, With<crate::game::BackgroundCamera>>,
    state: Res<GameState>,
) {
    let c = GameState::background_color(&state);

    camera.clear_color = ClearColorConfig::Custom(c.with_alpha(0.0).into());
}

use crate::scenes::Render;
use bevy::image::{ImageLoaderSettings, ImageSampler};

#[derive(Component)]
pub struct StaticSprite(String);

pub fn update_static_sprites(
    mut commands: Commands,
    assets: Res<AssetServer>,
    state: Res<GameState>,
    mut query: Query<(Entity, &mut Sprite, &mut Transform, &mut StaticSprite)>,
) {
    let sprites: Vec<StaticSpriteDescriptor> = GameState::sprites(&state).unwrap_or(vec![]);

    let mut sprite_entities: Vec<_> = query.iter_mut().collect();

    for (i, sprite) in sprites.iter().enumerate() {
        let pos = sprite.position.extend(sprite.z_index);
        let angle = sprite.angle;
        let scale = Vec3::splat(sprite.scale);
        let transform = Transform::from_scale(scale)
            .with_translation(pos)
            .with_rotation(Quat::from_rotation_z(angle));

        let (path, handle) = match sprite.path.clone() {
            SpritePath::Filesystem(path) => {
                let path = match std::fs::canonicalize(path.clone()) {
                    Ok(p) => p.to_string_lossy().to_string(),
                    Err(_) => path,
                };
                let handle = assets.load_with_settings(
                    path.clone(),
                    |settings: &mut ImageLoaderSettings| {
                        settings.sampler = ImageSampler::nearest();
                    },
                );
                (path, Some(handle))
            }
            SpritePath::Procedural(path) => {
                let handle = state.image_handles.get(&path).cloned();
                (path, handle)
            }
        };

        let handle = if let Some(handle) = handle {
            handle
        } else {
            Handle::default()
        };

        if let Some((_, ref mut spr, ref mut tf, ref mut desc)) = sprite_entities.get_mut(i) {
            **tf = transform;
            if desc.0 != path {
                **spr = Sprite::from_image(handle);
                println!("[{}] ({}) Modified sprite {}", i, state.wall_time, path);
                desc.0 = path.clone();
            }
        } else {
            println!("[{}] ({}) New sprite {}", i, state.wall_time, path);
            commands.spawn((
                Sprite::from_image(handle),
                transform,
                StaticSprite(path.clone()),
            ));
        }
    }

    for (i, (e, _, _, _)) in query.iter().enumerate() {
        if i >= sprites.len() {
            commands.entity(e).despawn();
        }
    }
}
