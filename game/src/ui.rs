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
    ToggleGraph,
    ClearSelection,
    ClearOrbitQueue,
    ExitApp,
    Save,
    Restore,
    Load(String),
    ToggleObject(ObjectId),
    ToggleGroup(GroupId),
    DisbandGroup(GroupId),
    CreateGroup(GroupId),
    ContextDependent,
    SelectionMode,

    // mouse stuff
    LeftMouseRelease,
    DoubleClick(Vec2),

    // camera operations
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    ZoomIn,
    ZoomOut,
    Reset,

    // manual piloting commands
    Thrust(i8, i8),
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(
            Update,
            (
                big_time_system,
                button_system,
                update_controller_buttons,
                update_constellation_buttons,
            ),
        );
    }
}

#[derive(Component, Debug, Copy, Clone)]
struct ControllerButton(ObjectId);

#[derive(Component, Debug, Clone)]
struct ConstellationButton(GroupId);

#[derive(Component, Debug, Copy, Clone)]
struct ControllerBar;

#[derive(Component, Debug, Copy, Clone)]
struct ConstellationBar;

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
            commands.entity(e).despawn_recursive();
        }
    }

    for ctrl in &state.controllers {
        if ctrl.is_idle() {
            continue;
        }
        if query.iter().find(|(_, cb)| cb.0 == ctrl.target).is_none() {
            let mut entity = None;
            commands.entity(*parent).with_children(|cb| {
                let e = add_ui_button(
                    cb,
                    &format!("{}", ctrl.target),
                    InteractionEvent::ToggleObject(ctrl.target),
                    false,
                    BLACK,
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

fn update_constellation_buttons(
    mut commands: Commands,
    parent: Single<Entity, With<ConstellationBar>>,
    query: Query<(Entity, &ConstellationButton)>,
    state: Res<GameState>,
) {
    for (e, cb) in &query {
        if state
            .constellations
            .iter()
            .find(|(c, _)| **c == cb.0)
            .is_none()
        {
            commands.entity(e).despawn_recursive();
        }
    }

    for (gid, _) in &state.constellations {
        if query.iter().find(|(_, cb)| &cb.0 == gid).is_none() {
            let mut entity = None;
            commands.entity(*parent).with_children(|cb| {
                let e = add_ui_button(
                    cb,
                    &format!("{}", gid),
                    InteractionEvent::ToggleGroup(gid.clone()),
                    false,
                    BLACK,
                );
                entity = Some(e);
            });
            if let Some(e) = entity {
                commands.entity(*parent).add_child(e);
                commands.entity(e).insert(ConstellationButton(gid.clone()));
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
            bottom: Val::Px(0.0),
            border: UiRect::all(Val::Px(2.0)),
            padding: UiRect::all(Val::Px(5.0)),
            column_gap: Val::Px(5.0),
            overflow: Overflow::clip_x(),
            ..default()
        },
        // BorderColor(BORDER_COLOR.into()),
        // BackgroundColor(BACKGROUND_COLOR.into()),
        ZIndex(100),
    )
}

fn add_ui_button(
    parent: &mut ChildBuilder<'_>,
    text: &str,
    onclick: InteractionEvent,
    holdable: bool,
    bg_color: Srgba,
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
        BackgroundColor(bg_color.into()),
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
        (">_", InteractionEvent::Console, false, BLACK),
        ("Debug", InteractionEvent::ToggleDebugMode, false, BLACK),
        (
            "Clear Tracks",
            InteractionEvent::ClearSelection,
            false,
            BLACK,
        ),
        (
            "Clear Orbits",
            InteractionEvent::ClearOrbitQueue,
            false,
            BLACK,
        ),
        ("Draw Orbits", InteractionEvent::Orbits, false, BLACK),
        ("Spawn", InteractionEvent::Spawn, true, BLACK),
        (
            "Commit Mission",
            InteractionEvent::CommitMission,
            false,
            DARK_GREEN.with_luminance(0.2),
        ),
        ("Reset Camera", InteractionEvent::Reset, false, BLACK),
        ("Del", InteractionEvent::Delete, false, BLACK),
        ("<", InteractionEvent::SimSlower, false, BLACK),
        ("||", InteractionEvent::SimPause, false, BLACK),
        (">", InteractionEvent::SimFaster, false, BLACK),
        ("Exit", InteractionEvent::ExitApp, false, BLACK),
        ("Save", InteractionEvent::Save, false, BLACK),
        ("Restore", InteractionEvent::Restore, false, BLACK),
        (
            "Load Earth",
            InteractionEvent::Load("earth".to_owned()),
            false,
            BLACK,
        ),
        (
            "Load Grid",
            InteractionEvent::Load("grid".to_owned()),
            false,
            BLACK,
        ),
        (
            "Load Luna",
            InteractionEvent::Load("moon".to_owned()),
            false,
            BLACK,
        ),
    ];

    commands.spawn(get_screen_clock());

    let top = commands.spawn(get_toplevel_ui()).id();

    let r1 = commands.spawn(get_ui_row()).insert(ConstellationBar).id();

    let r2 = commands.spawn(get_ui_row()).insert(ControllerBar).id();

    let r3 = commands
        .spawn(get_ui_row())
        .with_children(|parent| {
            for (name, event, holdable, color) in buttons {
                add_ui_button(parent, name, event, holdable, color);
            }
        })
        .id();

    commands.entity(top).add_children(&[r1, r2, r3]);
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
