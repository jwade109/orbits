use bevy::prelude::*;
use starling::prelude::*;
use std::time::Duration;

pub struct WarningsPlugin;

impl Plugin for WarningsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Events::<WarningEvent>::default());
        app.add_systems(
            Update,
            (spawn_new_warnings, send_random_warnings, update_warnings),
        );
    }
}

#[derive(Event)]
pub struct WarningEvent {
    pos: Vec2,
    message: String,
}

impl WarningEvent {
    pub fn new(pos: Vec2, message: impl Into<String>) -> Self {
        WarningEvent {
            pos,
            message: message.into(),
        }
    }
}

#[derive(Component)]
struct Warning(Duration, Vec2);

pub fn send_random_warnings(mut warnings: EventWriter<WarningEvent>) {
    if rand(0.0, 1.0) > 0.07 {
        return;
    }

    let warn = WarningEvent {
        pos: randvec(10.0, 10000.0),
        message: "Warning!".into(),
    };

    warnings.send(warn);
}

const WARNING_Z_INDEX: f32 = 9.0;

pub fn spawn_new_warnings(
    mut commands: Commands,
    mut warnings: EventReader<WarningEvent>,
    time: Res<Time>,
) {
    let stamp = time.elapsed();

    for warn in warnings.read() {
        commands.spawn((
            Warning(stamp, warn.pos),
            Transform::from_translation(warn.pos.extend(WARNING_Z_INDEX)),
            Text2d::new(warn.message.clone()),
            TextColor::WHITE,
        ));
    }
}

const WARNING_DUR: Duration = Duration::from_secs(3);

pub fn update_warnings(
    mut warnings: Query<(Entity, &mut Transform, &mut TextColor, &Warning)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let vel = Vec2::Y * 12.0;
    for (e, mut tf, mut color, warn) in &mut warnings {
        let dt = time.elapsed() - warn.0;
        let pos = warn.1 + vel * dt.as_secs_f32();
        tf.translation = pos.extend(WARNING_Z_INDEX);

        let a = 1.0 - dt.as_secs_f32() / WARNING_DUR.as_secs_f32();
        color.0 = color.0.with_alpha(a);

        if a < 0.0 {
            commands.entity(e).despawn();
        }
    }
}
