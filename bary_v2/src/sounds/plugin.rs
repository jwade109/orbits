use bary_core::prelude::{linspace, transform_to_isometry};
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use early_returns::ok_or_continue;

use crate::system_sets::DrawSet;

pub fn sounds_plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            draw_sound_emitter_locations.in_set(DrawSet),
            draw_spatial_listeners.in_set(DrawSet),
            add_sinks_to_indicator_flags,
        ),
    );
}

// TODO replace this with an enum or entity pointer
#[derive(Component, Debug)]
pub struct SoundSource(pub String);

fn add_sinks_to_indicator_flags(
    mut commands: Commands,
    query: Query<(Entity, &SoundSource), Without<PlaybackSettings>>,
    asset_server: Res<AssetServer>,
) {
    for (e, src) in query {
        let player = AudioPlayer::new(asset_server.load(src.0.clone()));
        let settings = PlaybackSettings::LOOP
            .with_spatial(true)
            .with_volume(bevy::audio::Volume::Linear(0.0));
        commands.entity(e).insert((player, settings));
        info!("Added spatial audio sink ({}) to entity {}", src.0, e);
    }
}

fn draw_sound_emitter_locations(
    mut gizmos: Gizmos,
    transforms: TransformHelper,
    emitters: Query<(Entity, &SpatialAudioSink)>,
) {
    for (e, sink) in emitters {
        let v = sink.volume().to_linear();
        if v == 0.0 {
            continue;
        }
        let tf = ok_or_continue!(transforms.compute_global_transform(e));
        let isometry = transform_to_isometry(tf.compute_transform());
        for r in linspace(0.2, 2.0, 5) {
            let r = r * v;
            gizmos.circle_2d(isometry, r, TEAL);
        }
    }
}

fn draw_spatial_listeners(
    mut gizmos: Gizmos,
    transforms: TransformHelper,
    listeners: Query<Entity, With<SpatialListener>>,
) {
    for e in listeners {
        let tf = ok_or_continue!(transforms.compute_global_transform(e));
        let isometry = transform_to_isometry(tf.compute_transform());
        for r in linspace(0.2, 1.0, 4) {
            gizmos.rect_2d(isometry, Vec2::splat(r), RED);
        }
    }
}
