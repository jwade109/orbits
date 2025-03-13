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
    ToggleController(ObjectId),

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
        app.add_systems(
            Update,
            (big_time_system, button_system, update_controller_buttons),
        );
    }
}

#[derive(Component, Debug, Copy, Clone)]
struct ControllerButton(ObjectId);

#[derive(Component, Debug, Copy, Clone)]
struct ControllerBar;

fn update_controller_buttons(
    mut commands: Commands,
    parent: Single<Entity, With<ControllerBar>>,
    query: Query<(Entity, &ControllerButton)>,
    state: Res<GameState>,
) {
    for (e, cb) in &query {
        if state
            .controllers
            .iter()
            .find(|c| c.target == cb.0)
            .is_none()
        {
            info!("Despawning controller button for {}", cb.0);
            commands.entity(e).despawn_recursive();
        }
    }

    for ctrl in &state.controllers {
        if query.iter().find(|(_, cb)| cb.0 == ctrl.target).is_none() {
            info!("Spawning controller button for {}", ctrl.target);
            let mut entity = None;
            commands.entity(*parent).with_children(|cb| {
                let e = add_ui_button(
                    cb,
                    &format!("C{}", ctrl.target),
                    InteractionEvent::ToggleController(ctrl.target),
                    false,
                );
                entity = Some(e);
            });
            if let Some(e) = entity {
                commands.entity(*parent).add_child(e);
                commands.entity(e).insert(ControllerButton(ctrl.target));
            };
        }
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

const BORDER_COLOR: Srgba = Srgba {
    alpha: 0.0,
    ..WHITE
};

const BACKGROUND_COLOR: Srgba = Srgba {
    alpha: 0.03,
    ..GRAY
};

fn get_toplevel_ui() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            border: UiRect::all(Val::Px(2.0)),
            padding: UiRect::all(Val::Px(5.0)),
            column_gap: Val::Px(5.0),
            row_gap: Val::Px(5.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::FlexEnd,
            ..default()
        },
        BorderColor(BORDER_COLOR.into()),
        ZIndex(100),
    )
}

fn get_ui_row() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Relative,
            width: Val::Percent(100.0),
            min_height: Val::Px(10.0),
            bottom: Val::Px(0.0),
            border: UiRect::all(Val::Px(2.0)),
            padding: UiRect::all(Val::Px(5.0)),
            column_gap: Val::Px(5.0),
            overflow: Overflow::clip_x(),
            ..default()
        },
        BorderColor(BORDER_COLOR.into()),
        BackgroundColor(BACKGROUND_COLOR.into()),
        ZIndex(100),
    )
}

fn add_ui_button(
    parent: &mut ChildBuilder<'_>,
    text: &str,
    onclick: InteractionEvent,
    holdable: bool,
) -> Entity {
    let mut entity = parent.spawn((
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
        BackgroundColor(BLACK.into()),
        OnClick(onclick, holdable),
        ZIndex(100),
    ));

    entity.with_child((
        Text::new(text),
        TextFont::from_font_size(20.0),
        TextColor(WHITE.into()),
        ZIndex(100),
    ));

    entity.id()
}

fn get_screen_clock() -> impl Bundle {
    (
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
    )
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

    commands.spawn(get_screen_clock());

    let top = commands.spawn(get_toplevel_ui()).id();

    let r1 = commands.spawn(get_ui_row()).insert(ControllerBar).id();

    let r2 = commands
        .spawn(get_ui_row())
        .with_children(|parent| {
            for (name, event, holdable) in buttons {
                add_ui_button(parent, name, event, holdable);
            }
        })
        .id();

    commands.entity(top).add_children(&[r1, r2]);
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
