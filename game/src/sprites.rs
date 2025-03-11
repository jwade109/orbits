use crate::planetary::GameState;
use bevy::asset::embedded_asset;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use starling::prelude::*;

pub struct PlanetSpritePlugin;

impl Plugin for PlanetSpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (add_background, spawn_box));
        app.add_systems(
            Update,
            (
                make_new_sprites,
                update_planet_sprites,
                update_shadow_sprites,
                update_background_sprite,
                update_spacecraft_sprites,
                button_system,
                big_time_system,
            ),
        );

        embedded_asset!(app, "src/", "../assets/Earth.png");
        embedded_asset!(app, "src/", "../assets/Luna.png");
        embedded_asset!(app, "src/", "../assets/Asteroid.png");
        embedded_asset!(app, "src/", "../assets/background.png");
        embedded_asset!(app, "src/", "../assets/shadow.png");
        embedded_asset!(app, "src/", "../assets/spacecraft.png");
    }
}

#[derive(Component)]
struct DateMarker;

fn spawn_box(mut commands: Commands) {
    commands.insert_resource(Events::<InteractionEvent>::default());

    let buttons = [
        (">_", InteractionEvent::Console),
        ("Debug", InteractionEvent::DebugMode),
        ("Clear", InteractionEvent::ClearSelection),
        ("Draw Orbits", InteractionEvent::Orbits),
        ("Spawn", InteractionEvent::Spawn),
        ("Commit Mission", InteractionEvent::CommitMission),
        ("Recenter", InteractionEvent::Recenter),
        ("Del", InteractionEvent::Delete),
        ("<", InteractionEvent::SimSlower),
        ("||", InteractionEvent::SimPause),
        (">", InteractionEvent::SimFaster),
        ("Exit", InteractionEvent::ExitApp),
    ];

    commands.spawn((
        DateMarker,
        Text::new(""),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(WHITE.into()),
        ZIndex(100),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0),
            ..default()
        },
    ));

    // ui camera
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(5.0)),
                margin: UiRect::all(Val::Px(5.0)),
                column_gap: Val::Px(5.0),
                ..default()
            },
            BorderColor(WHITE.with_alpha(0.04).into()),
            ZIndex(100),
        ))
        .with_children(|parent| {
            for (name, event) in buttons {
                parent
                    .spawn((
                        Button,
                        Node {
                            position_type: PositionType::Relative,
                            border: UiRect::all(Val::Px(2.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(5.0)),
                            ..default()
                        },
                        BorderColor(BLACK.into()),
                        BorderRadius::all(Val::Px(5.0)),
                        BackgroundColor(BLACK.into()),
                        OnClick(event),
                        ZIndex(100),
                    ))
                    .with_child((
                        Text::new(name),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(WHITE.into()),
                        ZIndex(100),
                    ));
            }
        });
}

#[derive(Debug, Event, Clone, Copy)]
pub enum InteractionEvent {
    Orbits,
    CommitMission,
    Spawn,
    Recenter,
    Console,
    Delete,
    SimSlower,
    SimPause,
    SimFaster,
    DebugMode,
    ClearSelection,
    Follow,
    ExitApp,
}

#[derive(Component, Debug, Clone, Copy)]
struct OnClick(InteractionEvent);

fn button_system(
    mut iq: Query<(&Interaction, &mut BorderColor, &OnClick), Changed<Interaction>>,
    mut evt: EventWriter<InteractionEvent>,
) {
    for (interaction, mut bc, OnClick(event)) in &mut iq {
        match *interaction {
            Interaction::Pressed => {
                bc.0 = ORANGE.into();
                evt.send(*event);
            }
            Interaction::Hovered => {
                bc.0 = WHITE.into();
            }
            Interaction::None => {
                bc.0 = GREY.into();
            }
        }
    }
}

fn big_time_system(mut q: Query<&mut Text, With<DateMarker>>, state: Res<GameState>) {
    const SCALE_FACTOR: i64 = Nanotime::PER_DAY / Nanotime::PER_SEC / 20;
    let t = state.sim_time * SCALE_FACTOR;
    for mut text in &mut q {
        let date = t.to_date();
        text.0 = format!(
            "Y{} W{} D{} {:02}:{:02}",
            date.year + 1,
            date.week + 1,
            date.day + 1,
            date.hour,
            date.min,
        );
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
struct BackgroundTexture;

#[derive(Component)]
#[require(Transform)]
struct PlanetTexture(ObjectId, String);

#[derive(Component)]
#[require(Transform)]
struct SpacecraftTexture(ObjectId, f32);

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
        commands.spawn((tf, SpacecraftTexture(id, 0.0), sprite));
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

const SPACECRAFT_DEFAULT_SCALE: f32 = 0.03;
const SPACECRAFT_MAGNIFIED_SCALE: f32 = 0.06;
const SPACECRAFT_DIMINISHED_SCALE: f32 = 0.01;

fn update_spacecraft_sprites(
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

fn update_background_sprite(
    mut query: Query<&mut Transform, With<BackgroundTexture>>,
    state: Res<GameState>,
) {
    let mut tf = query.single_mut();
    tf.translation = state.camera.center.extend(BACKGROUND_Z_INDEX);
    tf.scale = Vec3::splat(state.camera.actual_scale * 100.0);
}
