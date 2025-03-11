use crate::planetary::GameState;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use starling::prelude::*;

#[derive(Debug, Event, Clone)]
pub enum InteractionEvent {
    Orbits,
    CommitMission,
    Spawn,
    Console,
    Delete,
    SimSlower,
    SimPause,
    SimFaster,
    ToggleDebugMode,
    ClearSelection,
    Follow,
    ExitApp,
    ToggleSelectionMode,
    ToggleTargetMode,
    Save,
    Restore,
    Load(String),

    // mouse stuff
    DoubleClick,

    // camera operations
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    ZoomIn,
    ZoomOut,
    Reset,

    // manual piloting commands
    ThrustUp,
    ThrustDown,
    ThrustLeft,
    ThrustRight,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, (big_time_system, button_system));
    }
}

#[derive(Component, Debug, Clone)]
struct OnClick(InteractionEvent, bool);

#[derive(Component)]
struct DateMarker;

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

fn setup(mut commands: Commands) {
    commands.insert_resource(Events::<InteractionEvent>::default());

    let buttons = [
        (">_", InteractionEvent::Console, false),
        ("Debug", InteractionEvent::ToggleDebugMode, false),
        ("Clear", InteractionEvent::ClearSelection, false),
        ("Draw Orbits", InteractionEvent::Orbits, false),
        ("Spawn", InteractionEvent::Spawn, true),
        ("Commit Mission", InteractionEvent::CommitMission, false),
        ("Reset Camera", InteractionEvent::Reset, false),
        ("Del", InteractionEvent::Delete, false),
        ("<", InteractionEvent::SimSlower, false),
        ("||", InteractionEvent::SimPause, false),
        (">", InteractionEvent::SimFaster, false),
        ("Exit", InteractionEvent::ExitApp, false),
        ("Save", InteractionEvent::Save, false),
        ("Restore", InteractionEvent::Restore, false),
        (
            "Load Earth",
            InteractionEvent::Load("earth".to_owned()),
            false,
        ),
        (
            "Load Grid",
            InteractionEvent::Load("grid".to_owned()),
            false,
        ),
        (
            "Load Luna",
            InteractionEvent::Load("moon".to_owned()),
            false,
        ),
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
                max_width: Val::Percent(75.0),
                bottom: Val::Px(0.0),
                // border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(5.0)),
                margin: UiRect::all(Val::Px(5.0)),
                column_gap: Val::Px(5.0),
                ..default()
            },
            BorderColor(WHITE.with_alpha(0.02).into()),
            ZIndex(100),
        ))
        .with_children(|parent| {
            for (name, event, holdable) in buttons {
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
                        // BorderRadius::all(Val::Px(5.0)),
                        BackgroundColor(BLACK.into()),
                        OnClick(event, holdable),
                        ZIndex(100),
                    ))
                    .with_child((
                        Text::new(name),
                        TextFont::from_font_size(20.0),
                        TextColor(WHITE.into()),
                        ZIndex(100),
                    ));
            }
        });
}

fn button_system(
    mut iq: Query<(Ref<Interaction>, &mut BorderColor, &OnClick)>,
    mut evt: EventWriter<InteractionEvent>,
) {
    for (interaction, mut bc, OnClick(event, holdable)) in &mut iq {
        if interaction.is_changed() || *holdable {
            match *interaction {
                Interaction::Pressed => {
                    bc.0 = ORANGE.into();
                    evt.send(event.clone());
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
}
