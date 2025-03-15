use crate::planetary::GameState;
use bevy::asset::embedded_asset;
use bevy::prelude::*;
use starling::prelude::*;

pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_background);

        // app.add_systems(
        //     Update,
        //     (
        //         make_new_sprites,
        //         update_planet_sprites,
        //         update_shadow_sprites,
        //         update_background_sprite,
        //         update_spacecraft_sprites,
        //     ),
        // );

        embedded_asset!(app, "src/", "../assets/Earth.png");
        embedded_asset!(app, "src/", "../assets/Luna.png");
        embedded_asset!(app, "src/", "../assets/Asteroid.png");
        embedded_asset!(app, "src/", "../assets/background.png");
        embedded_asset!(app, "src/", "../assets/shadow.png");
        embedded_asset!(app, "src/", "../assets/spacecraft.png");
    }
}

const SELECTED_SPACECRAFT_Z_INDEX: f32 = 8.0;
const SHADOW_Z_INDEX: f32 = 7.0;
const SPACECRAFT_Z_INDEX: f32 = 6.0;
const PLANET_Z_INDEX: f32 = 5.0;
const BACKGROUND_Z_INDEX: f32 = 0.0;
const EXPECTED_PLANET_SPRITE_SIZE: u32 = 1000;

const EXPECTED_SHADOW_SPRITE_HEIGHT: u32 = 50;
const EXPECTED_SHADOW_SPRITE_WIDTH: u32 = 6000;

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

fn add_background(mut commands: Commands, asset_server: Res<AssetServer>) {
    let path = format!("embedded://game/../assets/background.png");
    let sprite = Sprite::from_image(asset_server.load(path));
    let t = Transform::from_scale(Vec2::splat(100000.0).extend(BACKGROUND_Z_INDEX));
    commands.spawn((BackgroundTexture, t, sprite));
}

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

                let sprite =
                    Sprite::from_image(asset_server.load("embedded://game/../assets/shadow.png"));
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
    mut query: Query<(Entity, &PlanetTexture, &mut Transform)>,
    state: Res<GameState>,
) {
    for (e, PlanetTexture(id, name), mut transform) in query.iter_mut() {
        let lup = match state.scenario.lup(*id, state.sim_time) {
            Some(lup) => lup,
            None => {
                commands.entity(e).despawn();
                continue;
            }
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
    mut query: Query<(Entity, &ShadowTexture, &mut Transform)>,
    state: Res<GameState>,
) {
    for (e, ShadowTexture(id), mut transform) in query.iter_mut() {
        let lup = match state.scenario.lup(*id, state.sim_time) {
            Some(lup) => lup,
            None => {
                commands.entity(e).despawn();
                continue;
            }
        };

        let body = match lup.body() {
            Some(b) => b,
            None => {
                commands.entity(e).despawn();
                continue;
            }
        };

        let angle = state.sim_time.to_secs() / 1000.0;
        let scale = (2.0 * body.radius) / EXPECTED_SHADOW_SPRITE_HEIGHT as f32;
        let w = EXPECTED_SHADOW_SPRITE_WIDTH as f32 * scale;
        let ds = rotate(Vec2::X * w / 2.0, angle);
        transform.translation = (lup.pv().pos + ds).extend(SHADOW_Z_INDEX);
        transform.scale = Vec3::new(scale, scale, 1.0);
        transform.rotation = Quat::from_rotation_z(angle);
    }
}

const SPACECRAFT_DEFAULT_SCALE: f32 = 0.015;
const SPACECRAFT_MAGNIFIED_SCALE: f32 = 0.06;
const SPACECRAFT_DIMINISHED_SCALE: f32 = 0.01;

pub fn update_spacecraft_sprites(
    mut commands: Commands,
    mut query: Query<(Entity, &mut SpacecraftTexture, &mut Transform)>,
    state: Res<GameState>,
) {
    for (e, mut x, mut transform) in query.iter_mut() {
        let SpacecraftTexture(id, scale) = *x;
        let lup = state.scenario.lup(id, state.sim_time);
        if let Some(lup) = lup {
            let z_index = if state.track_list.contains(&id) {
                SELECTED_SPACECRAFT_Z_INDEX
            } else {
                SPACECRAFT_Z_INDEX
            };
            transform.translation = lup.pv().pos.extend(z_index);
            let target_scale = if state.track_list.is_empty() {
                SPACECRAFT_DEFAULT_SCALE
            } else if state.track_list.contains(&id) {
                SPACECRAFT_MAGNIFIED_SCALE
            } else {
                SPACECRAFT_DIMINISHED_SCALE
            };
            transform.scale = Vec3::splat(scale * state.camera.actual_scale);
            x.1 += (target_scale - scale) * 0.1;
        } else {
            commands.entity(e).despawn();
        }
    }
}

pub fn update_background_sprite(
    mut query: Query<&mut Transform, With<BackgroundTexture>>,
    state: Res<GameState>,
) {
    let mut tf = query.single_mut();
    tf.translation = state.camera.world_center.extend(BACKGROUND_Z_INDEX);
    tf.scale = Vec3::splat(10000.0);
}
