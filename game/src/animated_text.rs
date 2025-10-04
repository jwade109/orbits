use bevy::color::palettes::css::*;
use bevy::prelude::*;
use starling::prelude::*;

pub struct AnimatedTextPlugin;

impl Plugin for AnimatedTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (receive_events, update_lifetimes));
        app.add_systems(FixedUpdate, update_text);
        app.add_event::<SpawnAnimText>();
    }
}

#[derive(Event)]
pub struct SpawnAnimText {
    pub text: String,
    pub color: Srgba,
}

#[derive(Component)]
struct AnimText(String);

#[derive(Component)]
struct Lifetime(f32);

fn receive_events(mut commands: Commands, mut events: EventReader<SpawnAnimText>) {
    for event in events.read() {
        let x = rand(15.0, 75.0);
        let y = rand(15.0, 75.0);

        commands.spawn((
            Text::new(""),
            AnimText(event.text.chars().into_iter().rev().collect()),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(x),
                left: Val::Percent(y),
                ..default()
            },
            Lifetime(rand(4.0, 6.0)),
            BackgroundColor(event.color.into()),
            BoxShadow::new(
                BLACK.with_alpha(0.9).into(),
                Val::Px(-5.0),
                Val::Px(5.0),
                Val::ZERO,
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
