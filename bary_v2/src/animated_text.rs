use std::collections::HashMap;

use bary_core::prelude::*;
use bevy::color::palettes::css::*;
use bevy::prelude::*;

pub struct AnimatedTextPlugin;

impl Plugin for AnimatedTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (receive_events, update_lifetimes));
        app.add_systems(FixedUpdate, update_text);
        app.add_message::<SpawnAnimText>();
    }
}

#[derive(Message)]
pub struct SpawnAnimText {
    pub text: String,
    pub color: Srgba,
    pub pos: Option<Vec2>,
    pub target: Option<Entity>,
}

impl SpawnAnimText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: BLUE,
            pos: None,
            target: None,
        }
    }
}

#[derive(Component)]
struct AnimText(String, Option<Entity>);

#[derive(Component)]
struct Lifetime(f32);

fn observer(
    trigger: On<Pointer<Click>>,
    mut query: Query<&mut BackgroundColor, With<AnimText>>,
    mut commands: Commands,
) {
    match trigger.button {
        PointerButton::Primary => (),
        PointerButton::Secondary => {
            commands.entity(trigger.entity).despawn();
            return;
        }
        PointerButton::Middle => return,
    }

    if let Ok(mut c) = query.get_mut(trigger.entity) {
        c.0 = DARK_GREEN.into();
        commands.entity(trigger.entity).remove::<Lifetime>();
    }
}

fn receive_events(mut commands: Commands, mut events: MessageReader<SpawnAnimText>) {
    for event in events.read() {
        let (x, y) = if let Some(pos) = event.pos {
            (Val::Px(pos.x), Val::Px(pos.y))
        } else {
            (
                Val::Percent(rand(15.0, 75.0)),
                Val::Percent(rand(15.0, 75.0)),
            )
        };

        commands
            .spawn((
                Text::new(""),
                AnimText(event.text.chars().into_iter().rev().collect(), event.target),
                Node {
                    position_type: PositionType::Absolute,
                    top: y,
                    left: x,
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
            ))
            .observe(observer);
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
