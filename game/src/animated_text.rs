use bevy::color::palettes::css::*;
use bevy::prelude::*;
use starling::prelude::*;

pub struct AnimatedTextPlugin;

impl Plugin for AnimatedTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_text_on_key,
                update_text,
                receive_events,
                update_lifetimes,
            ),
        );
        app.add_event::<SpawnAnimText>();
    }
}

#[derive(Event)]
pub struct SpawnAnimText;

#[derive(Component)]
pub struct AnimText(String);

#[derive(Component)]
pub struct Lifetime(f32);

fn spawn_text_on_key(mut events: EventWriter<SpawnAnimText>, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::KeyP) {
        events.write(SpawnAnimText);
    }
}

fn receive_events(mut commands: Commands, mut events: EventReader<SpawnAnimText>) {
    for _ in events.read() {
        let x = rand(15.0, 75.0);
        let y = rand(15.0, 75.0);

        let string: String = (0..randint(1, 9))
            .map(|_| "This is a very long label\n")
            .collect();

        commands.spawn((
            Text::new(""),
            AnimText(string.chars().into_iter().rev().collect()),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(x),
                left: Val::Percent(y),
                ..default()
            },
            Lifetime(rand(4.0, 6.0)),
            BackgroundColor(BLUE.mix(&WHITE, rand(0.2, 0.5)).into()),
            BoxShadow::new(
                BLACK.with_alpha(0.7).into(),
                Val::Px(-7.0),
                Val::Px(7.0),
                Val::Px(3.0),
                Val::ZERO,
            ),
        ));
    }
}

fn update_text(mut query: Query<(&mut AnimText, &mut Text)>) {
    for (mut anim, mut text) in &mut query {
        if let Some(c) = anim.0.pop() {
            text.0.push(c);
        }
    }
}

fn update_lifetimes(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Lifetime)>,
) {
    let dt = time.delta_secs();
    for (e, mut l) in &mut query {
        l.0 -= dt;
        if l.0 < 0.0 {
            commands.entity(e).despawn();
        }
    }
}
