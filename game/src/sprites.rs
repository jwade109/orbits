use crate::planetary::GameState;
use bevy::asset::embedded_asset;
use bevy::prelude::*;
use starling::prelude::*;

pub struct PlanetSpritePlugin;

impl Plugin for PlanetSpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_background);
        app.add_systems(
            Update,
            (
                make_new_sprites,
                update_planet_sprites,
                update_shadow_sprites,
                update_background_sprite,
                update_spacecraft_sprites,
            ),
        );

        embedded_asset!(app, "src/", "../assets/Earth.png");
        embedded_asset!(app, "src/", "../assets/Luna.png");
        embedded_asset!(app, "src/", "../assets/Asteroid.png");
        embedded_asset!(app, "src/", "../assets/spacecraft.png");
        embedded_asset!(app, "src/", "../assets/background.png");
        embedded_asset!(app, "src/", "../assets/shadow.png");
    }
}

const SHADOW_Z_INDEX: f32 = 7.0;
const SPACECRAFT_Z_INDEX: f32 = 6.0;
const PLANET_Z_INDEX: f32 = 5.0;
const BACKGROUND_Z_INDEX: f32 = 0.0;
const EXPECTED_PLANET_SPRITE_SIZE: u32 = 1000;

const EXPECTED_SHADOW_SPRITE_HEIGHT: u32 = 50;
const EXPECTED_SHADOW_SPRITE_WIDTH: u32 = 6000;

#[derive(Component)]
#[require(Transform)]
struct BackgroundTexture;

#[derive(Component)]
#[require(Transform)]
struct PlanetTexture(ObjectId, String);

#[derive(Component)]
#[require(Transform)]
struct SpacecraftTexture(ObjectId);

#[derive(Component)]
#[require(Transform)]
struct ShadowTexture(ObjectId);

fn add_background(mut commands: Commands, asset_server: Res<AssetServer>) {
    let path = format!("embedded://game/../assets/background.png");
    let sprite = Sprite::from_image(asset_server.load(path));
    let t = Transform::from_scale(Vec2::splat(100000.0).extend(BACKGROUND_Z_INDEX));
    commands.spawn((BackgroundTexture, t, sprite));
}

fn make_new_sprites(
    mut commands: Commands,
    ptextures: Query<&PlanetTexture>,
    stextures: Query<&SpacecraftTexture>,
    state: Res<GameState>,
    asset_server: Res<AssetServer>,
) {
    let planet_ids = state.scenario.system.ids();
    for id in planet_ids {
        if ptextures.iter().find(|e| e.0 == id).is_some() {
            continue;
        }
        let lup = state.scenario.system.lookup(id, state.sim_time);
        if let Some((_, _, _, sys)) = lup {
            let path = format!("embedded://game/../assets/{}.png", sys.name);
            println!("Adding sprite for {} at {}", sys.name, path);
            let sprite = Sprite::from_image(asset_server.load(path));
            commands.spawn((PlanetTexture(id, sys.name.clone()), sprite));

            let sprite =
                Sprite::from_image(asset_server.load("embedded://game/../assets/shadow.png"));
            commands.spawn((ShadowTexture(id), sprite));
        }
    }

    for id in state.scenario.ids() {
        if stextures.iter().find(|e| e.0 == id).is_some() {
            continue;
        }
        let path = "embedded://game/../assets/spacecraft.png";
        let sprite = Sprite::from_image(asset_server.load(path));
        let tf = Transform::from_scale(Vec3::ZERO);
        commands.spawn((tf, SpacecraftTexture(id), sprite));
    }
}

fn update_planet_sprites(
    mut commands: Commands,
    mut query: Query<(Entity, &PlanetTexture, &mut Transform)>,
    state: Res<GameState>,
) {
    for (e, PlanetTexture(id, name), mut transform) in query.iter_mut() {
        if let Some((body, pv, _, sys)) = state.scenario.system.lookup(*id, state.sim_time) {
            if sys.name == *name {
                transform.translation = pv.pos.extend(PLANET_Z_INDEX);
                transform.scale =
                    2.0 * Vec3::splat(body.radius) / EXPECTED_PLANET_SPRITE_SIZE as f32;
            } else {
                commands.entity(e).despawn();
            }
        } else {
            commands.entity(e).despawn();
        }
    }
}

fn update_shadow_sprites(
    mut commands: Commands,
    mut query: Query<(Entity, &ShadowTexture, &mut Transform)>,
    state: Res<GameState>,
) {
    for (e, ShadowTexture(id), mut transform) in query.iter_mut() {
        if let Some((body, pv, _, _)) = state.scenario.system.lookup(*id, state.sim_time) {
            let angle = state.sim_time.to_secs() / 1000.0;
            let scale = (2.0 * body.radius) / EXPECTED_SHADOW_SPRITE_HEIGHT as f32;
            let w = EXPECTED_SHADOW_SPRITE_WIDTH as f32 * scale;
            let ds = rotate(Vec2::X * w / 2.0, angle);
            transform.translation = (pv.pos + ds).extend(SHADOW_Z_INDEX);
            transform.scale = Vec3::new(scale, scale, 1.0);
            transform.rotation = Quat::from_rotation_z(angle)
        } else {
            commands.entity(e).despawn();
        }
    }
}

fn update_spacecraft_sprites(
    mut commands: Commands,
    mut query: Query<(Entity, &SpacecraftTexture, &mut Transform)>,
    state: Res<GameState>,
) {
    for (e, SpacecraftTexture(id), mut transform) in query.iter_mut() {
        let lup = state.scenario.lookup(*id, state.sim_time);
        if let Some(lup) = lup {
            transform.translation = lup.pv().pos.extend(SPACECRAFT_Z_INDEX);
            transform.scale = Vec3::splat(0.07 * state.camera.actual_scale.min(2.0));
        } else {
            commands.entity(e).despawn();
        }
    }
}

fn update_background_sprite(
    mut query: Query<&mut Transform, With<BackgroundTexture>>,
    state: Res<GameState>,
) {
    let mut tf = query.single_mut();
    tf.translation = state.camera.center.extend(BACKGROUND_Z_INDEX);
    tf.scale = Vec3::splat(state.camera.actual_scale * 100.0);
}
