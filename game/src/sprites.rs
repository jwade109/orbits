use crate::planetary::{GameMode, GameState};
use bevy::asset::embedded_asset;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use starling::math::is_occluded;
use starling::prelude::*;

pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "src/", "../assets/Earth.png");
        embedded_asset!(app, "src/", "../assets/Luna.png");
        embedded_asset!(app, "src/", "../assets/Asteroid.png");
        embedded_asset!(app, "src/", "../assets/shadow.png");
        embedded_asset!(app, "src/", "../assets/spacecraft.png");
    }
}

const SELECTED_SPACECRAFT_Z_INDEX: f32 = 8.0;
const SHADOW_Z_INDEX: f32 = 7.0;
const SPACECRAFT_Z_INDEX: f32 = 6.0;
const PLANET_Z_INDEX: f32 = 5.0;
const EXPECTED_PLANET_SPRITE_SIZE: u32 = 1000;

const EXPECTED_SHADOW_SPRITE_HEIGHT: u32 = 1000;

#[derive(Component)]
#[require(Transform)]
pub struct BackgroundTexture;

#[derive(Component)]
#[require(Transform)]
pub struct PlanetTexture(ObjectId, String);

#[derive(Component)]
#[require(Transform)]
pub struct SpacecraftTexture(ObjectId, f32);

#[derive(Component)]
#[require(Transform)]
pub struct ShadowTexture(ObjectId);

pub fn make_new_sprites(
    mut commands: Commands,
    ptextures: Query<&PlanetTexture>,
    stextures: Query<&SpacecraftTexture>,
    state: Res<GameState>,
    asset_server: Res<AssetServer>,
) {
    for id in state.scenario.planet_ids() {
        if ptextures.iter().find(|e| e.0 == id).is_some() {
            continue;
        }
        if let Some(lup) = state.scenario.lup(id, state.sim_time) {
            if let Some((name, _)) = lup.named_body() {
                let path = format!("embedded://game/../assets/{}.png", name);
                println!("Adding sprite for {} at {}", name, path);
                let sprite = Sprite::from_image(asset_server.load(path));
                commands.spawn((PlanetTexture(id, name.clone()), sprite));

                let mut sprite =
                    Sprite::from_image(asset_server.load("embedded://game/../assets/shadow.png"));
                sprite.color = RED.into();
                commands.spawn((ShadowTexture(id), sprite));
            }
        }
    }

    for id in state.scenario.orbiter_ids() {
        if stextures.iter().find(|e| e.0 == id).is_some() {
            continue;
        }
        let path = "embedded://game/../assets/spacecraft.png";
        let sprite = Sprite::from_image(asset_server.load(path));
        let tf = Transform::from_scale(Vec3::ZERO);
        commands.spawn((tf, SpacecraftTexture(id, 0.0), sprite));
    }
}

pub fn update_planet_sprites(
    mut commands: Commands,
    mut query: Query<(Entity, &PlanetTexture, &mut Transform, &mut Visibility)>,
    state: Res<GameState>,
) {
    for (e, PlanetTexture(id, name), mut transform, mut vis) in query.iter_mut() {
        let lup = match state.scenario.lup(*id, state.sim_time) {
            Some(lup) => lup,
            None => {
                commands.entity(e).despawn();
                continue;
            }
        };

        *vis = match state.game_mode {
            GameMode::Default => Visibility::Visible,
            _ => Visibility::Hidden,
        };

        let pv = lup.pv();
        let (lname, body) = match lup.named_body() {
            Some(n) => n,
            None => {
                commands.entity(e).despawn();
                continue;
            }
        };

        if lname == name {
            transform.translation = pv.pos.extend(PLANET_Z_INDEX);
            transform.scale = 2.0 * Vec3::splat(body.radius) / EXPECTED_PLANET_SPRITE_SIZE as f32;
        } else {
            commands.entity(e).despawn();
        }
    }
}

pub fn update_shadow_sprites(
    mut commands: Commands,
    mut query: Query<(Entity, &ShadowTexture, &mut Transform, &mut Visibility)>,
    state: Res<GameState>,
) {
    for (e, ShadowTexture(id), mut transform, mut vis) in query.iter_mut() {
        let lup = match state.scenario.lup(*id, state.sim_time) {
            Some(lup) => lup,
            None => {
                commands.entity(e).despawn();
                println!("Despawn shadow for {}", id);
                continue;
            }
        };

        *vis = match state.game_mode {
            GameMode::Default => Visibility::Visible,
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
        let scale = (2.0 * body.radius) / EXPECTED_SHADOW_SPRITE_HEIGHT as f32;
        transform.translation = (lup.pv().pos).extend(SHADOW_Z_INDEX);
        transform.scale = Vec3::new(scale, scale, 1.0);
        transform.rotation = Quat::from_rotation_z(angle);
    }
}

const SPACECRAFT_DEFAULT_SCALE: f32 = 0.025;
const SPACECRAFT_MAGNIFIED_SCALE: f32 = 0.06;
const SPACECRAFT_DIMINISHED_SCALE: f32 = 0.01;

pub fn hashable_to_color(h: &impl std::hash::Hash) -> Hsla {
    use std::hash::Hasher;
    let mut s = std::hash::DefaultHasher::new();
    h.hash(&mut s);
    let h: u64 = s.finish() % 1000;
    let hue = 360.0 * (h as f32 / 1000 as f32);
    Hsla::new(hue, 1.0, 0.5, 1.0)
}

pub fn update_spacecraft_sprites(
    mut commands: Commands,
    mut query: Query<(Entity, &mut SpacecraftTexture, &mut Transform, &mut Sprite)>,
    state: Res<GameState>,
) {
    let bodies: Vec<_> = state
        .scenario
        .planets()
        .bodies(state.sim_time, None)
        .collect();

    for (e, mut x, mut transform, mut s) in query.iter_mut() {
        let SpacecraftTexture(id, scale) = *x;
        let lup = state.scenario.lup(id, state.sim_time);
        let orbiter = lup.as_ref().map(|lup| lup.orbiter()).flatten();
        if let Some((lup, orbiter)) = lup.zip(orbiter) {
            let z_index = if state.track_list.contains(&id) {
                SELECTED_SPACECRAFT_Z_INDEX
            } else {
                SPACECRAFT_Z_INDEX
            };

            let pos = lup.pv().pos;

            transform.translation = pos.extend(z_index);

            let light_source = state.light_source();

            let is_lit = bodies
                .iter()
                .all(|(pv, body)| !is_occluded(light_source, pos, pv.pos, body.radius));

            let (target_scale, color) = if state.game_mode == GameMode::Default {
                let scale = if state.track_list.contains(&id) {
                    SPACECRAFT_MAGNIFIED_SCALE
                } else if !is_lit {
                    0.0
                } else if state.track_list.is_empty() {
                    SPACECRAFT_DEFAULT_SCALE
                } else {
                    SPACECRAFT_DIMINISHED_SCALE
                };

                let color = if state.track_list.is_empty() {
                    WHITE
                } else if state.track_list.contains(&id) {
                    WHITE
                } else {
                    WHITE.with_alpha(0.2)
                };

                (scale, color)
            } else if state.game_mode == GameMode::Constellations {
                let gid = match state.game_mode {
                    GameMode::Constellations => state.group_membership(&id),
                    _ => None,
                };

                let scale = if state.track_list.is_empty() && gid.is_some() {
                    SPACECRAFT_DEFAULT_SCALE
                } else if state.track_list.contains(&id) {
                    SPACECRAFT_MAGNIFIED_SCALE
                } else {
                    SPACECRAFT_DIMINISHED_SCALE
                };

                let color = if let Some(gid) = gid {
                    hashable_to_color(gid).into()
                } else {
                    WHITE.with_alpha(0.2)
                };

                (scale, color)
            } else if state.game_mode == GameMode::Stability {
                // stability
                let scale = if state.track_list.is_empty() {
                    SPACECRAFT_DEFAULT_SCALE
                } else if state.track_list.contains(&id) {
                    SPACECRAFT_MAGNIFIED_SCALE
                } else {
                    SPACECRAFT_DIMINISHED_SCALE
                };

                let color = match orbiter.is_indefinitely_stable() {
                    true => TEAL,
                    false => ORANGE,
                };

                (scale, color)
            } else {
                let scale = if state.track_list.is_empty() {
                    SPACECRAFT_DEFAULT_SCALE
                } else if state.track_list.contains(&id) {
                    SPACECRAFT_MAGNIFIED_SCALE
                } else {
                    SPACECRAFT_DIMINISHED_SCALE
                };

                let color = match is_lit {
                    true => WHITE,
                    false => RED,
                };

                (scale, color)
            };

            s.color = color.into();
            transform.scale = Vec3::splat(scale * state.camera.actual_scale);
            x.1 += (target_scale - scale) * 0.2;
        } else {
            commands.entity(e).despawn();
        }
    }
}

pub fn update_background_sprite(
    mut camera: Single<&mut Camera, With<crate::planetary::SoftController>>,
    state: Res<GameState>,
) {
    let c = match state.game_mode {
        GameMode::Default => BLACK,
        GameMode::Constellations => GRAY.with_luminance(0.1),
        GameMode::Stability => GRAY.with_luminance(0.13),
        GameMode::Occlusion => GRAY.with_luminance(0.04),
    };

    camera.clear_color = ClearColorConfig::Custom(c.into());
}
