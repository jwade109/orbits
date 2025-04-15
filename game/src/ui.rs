use crate::planetary::GameState;
use bevy::color::palettes::css::*;
use bevy::core_pipeline::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    view::RenderLayers,
};
use bevy::text::TextBounds;
use layout::layout as ui;
use starling::prelude::*;

#[allow(dead_code)]
#[derive(Debug, Event, Clone)]
pub enum InteractionEvent {
    Orbits,
    CommitMission,
    ClearMissions,
    Spawn,
    Console,
    Delete,
    SimSlower,
    SimPause,
    SimFaster,
    ToggleGraph,
    ClearSelection,
    ClearOrbitQueue,
    ExitApp,
    Save,
    Restore,
    Load(String),
    ToggleObject(OrbiterId),
    ToggleGroup(GroupId),
    DisbandGroup(GroupId),
    CreateGroup,
    ContextDependent,
    CursorMode,
    GameMode,
    RedrawGui,
    ToggleFullscreen,

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
    ThrustForward,
    TurnLeft,
    TurnRight,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(
            Update,
            (
                big_time_system,
                do_ui_sprites,
                top_right_text_system,
                set_bloom,
            ),
        );
    }
}

fn set_bloom(state: Res<GameState>, mut bloom: Single<&mut Bloom>) {
    bloom.intensity = match state.game_mode {
        crate::planetary::GameMode::Default => 0.3,
        _ => 0.1,
    }
}

const TEXT_LABEL_Z_INDEX: f32 = 10.0;

pub fn do_text_labels(
    mut commands: Commands,
    state: Res<GameState>,
    mut query: Query<(Entity, &mut Text2d, &mut Transform), With<TextLabel>>,
) {
    let mut labels: Vec<_> = query.iter_mut().collect();
    for (i, (pos, txt, size)) in state.text_labels.iter().enumerate() {
        if let Some((_, text2d, label)) = labels.get_mut(i) {
            label.translation = pos.extend(TEXT_LABEL_Z_INDEX);
            label.scale = Vec3::splat(*size);
            text2d.0 = txt.clone();
        } else {
            commands.spawn((
                Text2d::new(txt.clone()),
                Transform::from_translation(pos.extend(TEXT_LABEL_Z_INDEX))
                    .with_scale(Vec3::splat(*size)),
                TextLabel,
            ));
        }
    }

    for (i, (e, _, _)) in query.iter().enumerate() {
        if i >= state.text_labels.len() {
            commands.entity(e).despawn();
        }
    }
}

#[derive(Component)]
pub struct TextLabel;

#[derive(Component)]
struct DateMarker;

#[derive(Component)]
struct TopRight;

fn big_time_system(mut text: Single<&mut Text, With<DateMarker>>, state: Res<GameState>) {
    const SCALE_FACTOR: i64 = Nanotime::PER_DAY / Nanotime::PER_SEC / 20;
    let t = state.sim_time * SCALE_FACTOR;
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

fn top_right_text_system(mut text: Single<&mut Text, With<TopRight>>, state: Res<GameState>) {
    let res = (|| -> Option<(&Orbiter, GlobalOrbit)> {
        let id = state.follow?.orbiter()?;
        let orbiter = state.scenario.lup_orbiter(id, state.sim_time)?.orbiter()?;
        let prop = orbiter.propagator_at(state.sim_time)?;
        let go = prop.orbit;
        Some((orbiter, go))
    })();

    if let Some((orbiter, go)) = res {
        text.0 = format!("{}\nOrbit: {}", orbiter, go);
    } else {
        text.0 = "".into();
    }
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

fn get_top_right_ui() -> impl Bundle {
    (
        TopRight,
        Text::new(""),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(WHITE.into()),
        ZIndex(100),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(70.0),
            right: Val::Px(30.0),
            ..default()
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuiNodeId {
    Orbiter(OrbiterId),
    Exit,
    Save,
    Load,
    ToggleDrawMode,
    ClearTracks,
    CreateGroup,
    DisbandGroup(GroupId),
    ClearOrbits,
    CurrentBody(PlanetId),
    SelectedCount,
    AutopilotingCount,
    PilotOrbiter,
    Group(GroupId),
    TogglePause,
    World,
    SimSpeed(i32),
    GlobalOrbit(usize),
    DeleteOrbit(usize),
    DeleteOrbiter,
    ClearMission,
    CommitMission,
    FollowOrbiter,
    CursorMode,
    Scene(usize),
    Nullopt,
}

pub fn context_menu(rowsize: f32, items: &[(String, GuiNodeId, bool)]) -> ui::Node<GuiNodeId> {
    use ui::*;
    Node::new(200, Size::Fit)
        .down()
        .with_color([0.1, 0.1, 0.1, 1.0])
        .with_children(items.iter().map(|(s, id, e)| {
            Node::button(s, id.clone(), Size::Grow, rowsize)
                .with_color([0.3, 0.3, 0.3, 1.0])
                .enabled(*e)
        }))
}

pub fn orbiter_context_menu(id: OrbiterId) -> ui::Node<GuiNodeId> {
    context_menu(
        30.0,
        &[
            (format!("Orbiter {}", id), GuiNodeId::Orbiter(id), false),
            ("Delete".into(), GuiNodeId::DeleteOrbiter, true),
            ("Pilot".into(), GuiNodeId::PilotOrbiter, true),
            ("Clear Mission".into(), GuiNodeId::ClearMission, true),
            ("Follow".into(), GuiNodeId::FollowOrbiter, true),
        ],
    )
}

pub const DELETE_SOMETHING_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 0.5];
pub const UI_BACKGROUND_COLOR: [f32; 4] = [0.1, 0.1, 0.1, 0.7];

fn delete_wrapper(
    ondelete: GuiNodeId,
    button: ui::Node<GuiNodeId>,
    width: impl Into<ui::Size>,
    height: impl Into<ui::Size>,
) -> ui::Node<GuiNodeId> {
    let height = height.into();
    let x_button = {
        let s = "X";
        ui::Node::button(s, ondelete, height, height).with_color(DELETE_SOMETHING_COLOR)
    };

    ui::Node::new(width.into(), height)
        .tight()
        .invisible()
        .with_child(x_button)
        .with_child(button)
}

pub fn layout(state: &GameState) -> Option<ui::Tree<GuiNodeId>> {
    use ui::*;

    let small_button_height = 30;
    let button_height = 40;

    let vb = state.camera.viewport_bounds();
    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return None;
    }

    let topbar = Node::row(Size::Fit)
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(Node::button("Save", GuiNodeId::Save, 80, Size::Grow))
        .with_child(Node::button("Load", GuiNodeId::Load, 80, Size::Grow))
        .with_children((0..4).map(|_| Node::new(80, small_button_height)))
        .with_child(Node::grow().invisible())
        .with_child(Node::button("Exit", GuiNodeId::Exit, 80, Size::Grow));

    let mut sidebar = Node::column(300).with_color(UI_BACKGROUND_COLOR);

    let body_color_lup: std::collections::HashMap<&'static str, Srgba> =
        std::collections::HashMap::from([("Earth", BLUE), ("Luna", GRAY), ("Asteroid", BROWN)]);

    if let Some(lup) = state
        .scenario
        .relevant_body(state.camera.world_center, state.sim_time)
        .map(|id| state.scenario.lup_planet(id, state.sim_time))
        .flatten()
    {
        if let Some((s, _)) = lup.named_body() {
            let color: Srgba = body_color_lup
                .get(s.as_str())
                .unwrap_or(&Srgba::from(crate::sprites::hashable_to_color(s)))
                .with_luminance(0.2)
                .with_alpha(0.9);

            sidebar.add_child(
                Node::button(
                    s,
                    GuiNodeId::CurrentBody(lup.id().planet().unwrap()),
                    Size::Grow,
                    button_height,
                )
                // .enabled(false)
                .with_color(color.to_f32_array()),
            );
        }
    }

    sidebar.add_child(Node::button(
        format!("Visual: {:?}", state.game_mode),
        GuiNodeId::ToggleDrawMode,
        Size::Grow,
        button_height,
    ));

    sidebar.add_child(
        Node::button(
            "Clear Orbits",
            GuiNodeId::ClearOrbits,
            Size::Grow,
            button_height,
        )
        .enabled(!state.queued_orbits.is_empty()),
    );

    sidebar.add_child(
        Node::button(
            "Commit Mission",
            GuiNodeId::CommitMission,
            Size::Grow,
            button_height,
        )
        .enabled(!state.queued_orbits.is_empty() && !state.track_list.is_empty()),
    );

    sidebar.add_child(Node::button(
        format!("Cursor: {:?}", state.selection_mode),
        GuiNodeId::CursorMode,
        Size::Grow,
        button_height,
    ));

    if !state.constellations.is_empty() {
        sidebar.add_child(Node::hline());
    }

    for gid in state.unique_groups() {
        let color: Srgba = crate::sprites::hashable_to_color(gid)
            .with_luminance(0.3)
            .into();
        let s = format!("{}", gid);
        let id = GuiNodeId::Group(gid.clone());
        let button =
            Node::button(s, id, Size::Grow, button_height).with_color(color.to_f32_array());
        let wrapper = delete_wrapper(
            GuiNodeId::DisbandGroup(gid.clone()),
            button,
            Size::Grow,
            button_height as f32,
        );
        sidebar.add_child(wrapper);
    }

    sidebar.add_child(Node::hline());

    sidebar.add_child({
        let s = format!("{} selected", state.track_list.len());
        let b = Node::button(s, GuiNodeId::SelectedCount, Size::Grow, button_height).enabled(false);
        if state.track_list.is_empty() {
            b
        } else {
            delete_wrapper(GuiNodeId::ClearTracks, b, Size::Grow, button_height)
        }
    });

    let orbiter_list = |root: &mut Node<GuiNodeId>, max_cells: usize, mut ids: Vec<OrbiterId>| {
        ids.sort();

        let rows = (ids.len().min(max_cells) as f32 / 4.0).ceil() as u32;
        let grid = Node::grid(Size::Grow, rows * button_height, rows, 4, 4.0, |i| {
            if i as usize > max_cells {
                return None;
            }
            let id = ids.get(i as usize)?;
            let s = format!("{id}");
            Some(
                Node::grow()
                    .with_id(GuiNodeId::Orbiter(*id))
                    .with_text(s)
                    .enabled(Some(*id) != state.follow.map(|f| f.orbiter()).flatten()),
            )
        });
        root.add_child(grid);

        if ids.len() > max_cells {
            let n = ids.len() - max_cells;
            let s = format!("...And {} more", n);
            root.add_child(
                Node::new(Size::Grow, button_height)
                    .with_text(s)
                    .enabled(false),
            );
        }
    };

    if !state.track_list.is_empty() {
        orbiter_list(&mut sidebar, 32, state.track_list.iter().cloned().collect());
        sidebar.add_child(Node::button(
            "Create Group",
            GuiNodeId::CreateGroup,
            Size::Grow,
            button_height,
        ));
    }

    if !state.controllers.is_empty() {
        sidebar.add_child(Node::hline());
        let s = format!("{} autopiloting", state.controllers.len());
        let id = GuiNodeId::AutopilotingCount;
        sidebar.add_child(Node::button(s, id, Size::Grow, button_height).enabled(false));

        let ids = state.controllers.iter().map(|c| c.target()).collect();
        orbiter_list(&mut sidebar, 16, ids);
    }

    let mut inner_topbar = Node::fit()
        .with_padding(0.0)
        .invisible()
        .with_id(GuiNodeId::World)
        .with_child({
            let s = if state.paused { "UNPAUSE" } else { "PAUSE" };
            Node::button(s, GuiNodeId::TogglePause, 120, button_height)
        })
        .with_children((-2..=2).map(|i| {
            Node::button(
                format!("{i}"),
                GuiNodeId::SimSpeed(i),
                button_height,
                button_height,
            )
            .enabled(i != state.sim_speed)
        }));

    if let Some(id) = state.follow {
        let s = format!("Following {}", id);
        let id = GuiNodeId::Nullopt;
        let n = Node::button(s, id, 180, button_height).enabled(false);
        inner_topbar.add_child(n);
    }

    for (i, orbit) in state.queued_orbits.iter().enumerate() {
        let orbit_button = {
            let s = format!("{}", orbit);
            let id = GuiNodeId::GlobalOrbit(i);
            Node::button(s, id, 400, button_height)
        };

        let n = delete_wrapper(
            GuiNodeId::DeleteOrbit(i),
            orbit_button,
            Size::Fit,
            button_height as f32,
        );

        inner_topbar.add_child(n);
    }

    let notif_bar = Node::fit().down().tight().invisible().with_children(
        state.notifications.iter().rev().take(10).rev().map(|n| {
            let s = format!("{}", n);
            ui::Node::new(900, 28)
                .with_text(s)
                .with_color([0.3, 0.3, 0.3, 0.3])
        }),
    );

    let scene_bar = Node::row(Size::Fit)
        .with_id(GuiNodeId::World)
        .invisible()
        .with_padding(0.0)
        .with_children(
            state
                .scenes
                .iter()
                .enumerate()
                .map(|(i, s)| Node::button(s.name(), GuiNodeId::Scene(i), 180, button_height)),
        );

    let world = Node::grow()
        .down()
        .invisible()
        .with_id(GuiNodeId::World)
        .with_child(
            Node::grow()
                .with_id(GuiNodeId::World)
                .tight()
                .invisible()
                .with_child(inner_topbar),
        )
        .with_child(
            Node::grow()
                .with_id(GuiNodeId::World)
                .tight()
                .down()
                .invisible()
                .with_child(Node::grow().with_id(GuiNodeId::World).invisible())
                .with_child(notif_bar)
                .with_child(Node::row(15.0).with_id(GuiNodeId::World).invisible())
                .with_child(scene_bar),
        );

    let root = Node::new(vb.span.x, vb.span.y)
        .down()
        .tight()
        .invisible()
        .with_child(topbar)
        .with_child(
            Node::grow()
                .tight()
                .invisible()
                .with_child(sidebar)
                .with_child(world),
        );

    let mut tree = Tree::new().with_layout(root, Vec2::ZERO);

    if let Some(p) = state.context_menu_origin {
        let ctx = orbiter_context_menu(OrbiterId(0));
        let p = Vec2::new(p.x, state.camera.viewport_bounds().span.y - p.y);
        tree.add_layout(ctx, p);
    }

    Some(tree)
}

#[derive(Component)]
struct UiElement;

fn generate_button_sprite(node: &layout::layout::Node<GuiNodeId>) -> Image {
    let aabb = node.aabb();
    let w = (aabb.span.x as u32).max(1);
    let h = (aabb.span.y as u32).max(1);

    let color = node.color();
    let color = Srgba::new(color[0], color[1], color[2], color[3]);

    let mut image = Image::new_fill(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &color.to_u8_array(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    image.sampler = bevy::image::ImageSampler::nearest();

    // if w != 1 && h != 1 && node.is_leaf() {
    //     for (x, y) in [(0, 0), (0, h - 1), (w - 1, 0), (w - 1, h - 1)] {
    //         if let Some(bytes) = image.pixel_bytes_mut(UVec3::new(x, y, 0)) {
    //             bytes[3] = 0;
    //         }
    //     }
    // }

    image
}

fn do_ui_sprites(
    mut commands: Commands,
    to_despawn: Query<Entity, With<UiElement>>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<GameState>,
) {
    let ui_age = state.wall_time - state.last_redraw;

    if ui_age < Nanotime::millis(50) {
        return;
    }

    if ui_age < Nanotime::secs(1) && !state.redraw_requested {
        return;
    }

    let vb = state.camera.viewport_bounds();

    for e in &to_despawn {
        commands.entity(e).despawn();
    }

    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return;
    }

    state.ui = match layout(&state) {
        Some(ui) => ui,
        None => ui::Tree::new(),
    };

    state.last_redraw = state.wall_time;
    state.redraw_requested = false;

    for (lid, layout) in state.ui.layouts().iter().enumerate() {
        for n in layout.iter() {
            if !n.is_visible() {
                continue;
            }

            let image = generate_button_sprite(n);
            let aabb = n.aabb();

            let mut c = aabb.center;

            c.x -= vb.span.x / 2.0;
            c.y = vb.span.y / 2.0 - c.y;

            let transform =
                Transform::from_translation(c.extend(n.layer() as f32 / 100.0 + lid as f32));

            let handle = images.add(image);

            commands.spawn((
                transform,
                Sprite::from_image(handle.clone()),
                RenderLayers::layer(1),
                UiElement,
            ));

            if n.is_leaf() {
                let bounds = TextBounds {
                    width: Some(aabb.span.x),
                    height: Some(aabb.span.y),
                };

                let mut transform = transform;
                transform.translation.z += 0.01;
                if let Some(s) = n.text_content() {
                    commands.spawn((
                        transform,
                        bounds,
                        Text2d::new(s.to_uppercase()),
                        RenderLayers::layer(1),
                        UiElement,
                    ));
                }
            }
        }
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(Events::<InteractionEvent>::default());
    commands.spawn(get_screen_clock());
    commands.spawn(get_top_right_ui());
}
