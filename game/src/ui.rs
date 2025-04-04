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
    CreateGroup,
    ContextDependent,
    SelectionMode,
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
                do_ui_sprites,
                top_right_text_system,
                set_bloom,
            ),
        );
    }
}

fn set_bloom(state: Res<GameState>, mut bloom: Single<&mut Bloom>) {
    bloom.intensity = match state.game_mode {
        crate::planetary::GameMode::Default => 0.6,
        _ => 0.0,
    }
}

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
        let fid = state.follow?;
        if !state.track_list.contains(&fid) {
            return None;
        }
        let orbiter = state.scenario.lup(fid, state.sim_time)?.orbiter()?;
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
    Orbiter(ObjectId),
    Exit,
    ToggleDrawMode,
    ClearTracks,
    CreateGroup,
    ClearOrbits,
    CurrentBody,
    SelectedCount,
    AutopilotingCount,
    PilotOrbiter,
    Group(GroupId),
    ToggleDebug,
    TogglePause,
    World,
    SimSpeed,
    GlobalOrbit(usize),
    DeleteOrbiter,
    ClearMission,
    FollowOrbiter,
}

pub fn context_menu(rowsize: f32, items: &[(String, GuiNodeId, bool)]) -> ui::Node<GuiNodeId> {
    use ui::*;
    Node::new(200, Size::Fit).down().with_children(
        items
            .iter()
            .map(|(s, id, e)| Node::button(s, id.clone(), Size::Grow, rowsize).enabled(*e)),
    )
}

pub fn orbiter_context_menu(id: ObjectId) -> ui::Node<GuiNodeId> {
    context_menu(
        40.0,
        &[
            (format!("Orbiter {}", id), GuiNodeId::Orbiter(id), false),
            ("Delete".into(), GuiNodeId::DeleteOrbiter, true),
            ("Pilot".into(), GuiNodeId::PilotOrbiter, true),
            ("Clear Mission".into(), GuiNodeId::ClearMission, true),
            ("Follow".into(), GuiNodeId::FollowOrbiter, true),
        ],
    )
}

pub fn layout(state: &GameState) -> Option<ui::Tree<GuiNodeId>> {
    use ui::*;

    let small_button_height = 30;
    let button_height = 40;

    let vb = state.camera.viewport_bounds();
    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return None;
    }

    let mut tracked_ids: Vec<_> = state.track_list.iter().collect();

    tracked_ids.sort();

    let topbar = Node::row(Size::Fit)
        .with_children((0..5).map(|_| Node::new(80, small_button_height)))
        .with_child(Node::grow().invisible())
        .with_child(Node::button("Exit", GuiNodeId::Exit, 80, Size::Grow));

    let mut sidebar = Node::column(300);

    if let Some((s, _)) = state
        .scenario
        .relevant_body(state.camera.world_center, state.sim_time)
        .map(|id| state.scenario.lup(id, state.sim_time))
        .flatten()
        .map(|lup| lup.named_body())
        .flatten()
    {
        sidebar.add_child(
            Node::button(s, GuiNodeId::CurrentBody, Size::Grow, button_height).enabled(false),
        );
    }

    sidebar.add_child({
        let s = if !state.hide_debug {
            "Hide Debug Info"
        } else {
            "Show Debug Info"
        };
        Node::button(s, GuiNodeId::ToggleDebug, Size::Grow, button_height)
    });

    sidebar.add_child(Node::button(
        "Visual Mode",
        GuiNodeId::ToggleDrawMode,
        Size::Grow,
        button_height,
    ));

    sidebar.add_child(
        Node::button(
            "Clear Tracks",
            GuiNodeId::ClearTracks,
            Size::Grow,
            button_height,
        )
        .enabled(!state.track_list.is_empty()),
    );

    sidebar.add_child(
        Node::button(
            "Clear Orbits",
            GuiNodeId::ClearOrbits,
            Size::Grow,
            button_height,
        )
        .enabled(!state.queued_orbits.is_empty()),
    );

    if !state.constellations.is_empty() {
        sidebar.add_child(Node::hline());
    }

    for gid in state.unique_groups() {
        let s = format!("GID {}", gid);
        let id = GuiNodeId::Group(gid.clone());
        sidebar.add_child(Node::button(s, id, Size::Grow, button_height));
    }

    sidebar.add_child(Node::hline());

    sidebar.add_child({
        let s = format!("{} selected", state.track_list.len());
        Node::button(s, GuiNodeId::SelectedCount, Size::Grow, button_height).enabled(false)
    });

    if !state.track_list.is_empty() {
        let max_cells = 32;
        let tracks = state.track_list.iter().collect::<Vec<_>>();
        let rows = (tracks.len().min(max_cells) as f32 / 4.0).ceil() as u32;
        let grid = Node::grid(Size::Grow, rows * button_height, rows, 4, 4.0, |i| {
            if i as usize > max_cells {
                return None;
            }
            let id = tracks.get(i as usize)?;
            let s = format!("{id}");
            Some(Node::grow().with_id(GuiNodeId::Orbiter(**id)).with_text(s))
        });
        sidebar.add_child(grid);

        if state.track_list.len() > max_cells {
            let n = state.track_list.len() - max_cells;
            let s = format!("...And {} more", n);
            sidebar.add_child(
                Node::new(Size::Grow, button_height)
                    .with_text(s)
                    .enabled(false),
            );
        }

        sidebar.add_child(Node::button(
            "Create Group",
            GuiNodeId::CreateGroup,
            Size::Grow,
            button_height,
        ));
    }

    if let Some(fid) = state.follow {
        if state.track_list.contains(&fid) {
            sidebar.add_child(Node::hline());
            let s = format!("Pilot {}", fid);
            let id = GuiNodeId::PilotOrbiter;
            sidebar.add_child(Node::button(s, id, Size::Grow, button_height));
        }
    }

    if !state.controllers.is_empty() {
        sidebar.add_child(Node::hline());
        let s = format!("{} autopiloting", state.controllers.len());
        let id = GuiNodeId::AutopilotingCount;
        sidebar.add_child(Node::button(s, id, Size::Grow, button_height).enabled(false));
    }

    let mut world = Node::grow()
        .invisible()
        .with_id(GuiNodeId::World)
        .with_child({
            let s = if state.paused { "UNPAUSE" } else { "PAUSE" };
            Node::button(s, GuiNodeId::TogglePause, 120, button_height)
        })
        .with_children((-2..=2).map(|i| {
            Node::button(
                format!("{i}"),
                GuiNodeId::SimSpeed,
                button_height,
                button_height,
            )
            .enabled(i != state.sim_speed)
        }));

    for (i, orbit) in state.queued_orbits.iter().enumerate() {
        let s = format!("{}", orbit);
        let id = GuiNodeId::GlobalOrbit(i);
        world.add_child(Node::button(s, id, 400, button_height));
    }

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
        let ctx = orbiter_context_menu(ObjectId(0));
        let p = Vec2::new(p.x, state.camera.viewport_bounds().span.y - p.y);
        tree.add_layout(ctx, p);
    }

    Some(tree)
}

#[derive(Component)]
struct UiElement;

fn generate_button_sprite(node: &layout::layout::Node<GuiNodeId>, color: Option<Srgba>) -> Image {
    let aabb = node.aabb();
    let w = (aabb.span.x as u32).max(1);
    let h = (aabb.span.y as u32).max(1);

    let color = color.unwrap_or(if node.is_leaf() && node.is_enabled() {
        ORANGE.with_luminance(0.4).with_alpha(1.0)
    } else if node.is_leaf() {
        GRAY.with_luminance(0.3).with_alpha(0.4)
    } else {
        BLACK.with_alpha(0.8)
    });

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

    if w != 1 && h != 1 && node.is_leaf() {
        for y in 0..h {
            for x in 0..w {
                if x < 3 || y < 3 || x > w - 4 || y > h - 4 {
                    if let Some(bytes) = image.pixel_bytes_mut(UVec3::new(x, y, 0)) {
                        bytes[3] = 0;
                    }
                }
            }
        }

        for (x, y) in [(0, 0), (0, h - 1), (w - 1, 0), (w - 1, h - 1)] {
            if let Some(bytes) = image.pixel_bytes_mut(UVec3::new(x, y, 0)) {
                bytes[3] = 0;
            }
        }
    }

    image
}

fn do_ui_sprites(
    mut commands: Commands,
    to_despawn: Query<Entity, With<UiElement>>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<GameState>,
) {
    if state.actual_time - state.last_redraw < Nanotime::millis(1000) {
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

    state.last_redraw = state.actual_time;

    for (lid, layout) in state.ui.layouts().iter().enumerate() {
        for n in layout.iter() {
            if !n.is_visible() {
                continue;
            }

            let color: Option<Srgba> = if let Some(GuiNodeId::Group(gid)) = n.id() {
                Some(
                    crate::sprites::hashable_to_color(gid)
                        .with_luminance(0.3)
                        .into(),
                )
            } else {
                None
            };

            let image = generate_button_sprite(n, color);
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
