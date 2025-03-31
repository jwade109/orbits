use crate::planetary::GameState;
use bevy::color::palettes::css::*;
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
            (big_time_system, do_ui_sprites, top_right_text_system),
        );
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
            top: Val::Px(20.0),
            right: Val::Px(5.0),
            ..default()
        },
    )
}

pub fn context_menu(n: u32, m: u32) -> ui::Node {
    use ui::*;
    let spacing = 4.0;
    Node::new(200, 300)
        .down()
        .with_child(Node::row(20))
        .with_child(Node::row(40))
        .with_child(Node::grid(Size::Grow, Size::Grow, n, m, spacing))
}

pub fn layout(state: &GameState) -> Option<ui::Tree> {
    use ui::*;

    let small_button_height = 30;
    let button_height = 40;

    let vb = state.camera.viewport_bounds();
    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return None;
    }

    // let mut buttons: Vec<(String, String)> = vec![
    //     ("Commit Mission".into(), "commit-mission".into()),
    //     ("Clear Orbits".into(), "clear-orbits".into()),
    //     ("Warp to Periapsis".into(), "warp-to-periapsis".into()),
    // ];

    let mut tracked_ids: Vec<_> = state.track_list.iter().collect();

    tracked_ids.sort();

    let topbar = Node::row(Size::Fit)
        .with_children((0..5).map(|_| Node::new(80, small_button_height)))
        .with_child(Node::grow())
        .with_child(Node::button("Exit", "exit", 80, Size::Grow));

    let mut sidebar = Node::column(300);

    sidebar.add_child({
        let s = if !state.hide_debug {
            "Hide Debug Info"
        } else {
            "Show Debug Info"
        };
        Node::button(s, "toggle-debug", Size::Grow, button_height)
    });

    sidebar.add_child(Node::button(
        "Visual Mode",
        "toggle-visual-mode",
        Size::Grow,
        button_height,
    ));

    sidebar.add_child(
        Node::button("Clear Tracks", "clear-tracks", Size::Grow, button_height)
            .enabled(!state.track_list.is_empty()),
    );

    sidebar.add_child(
        Node::button("Clear Orbits", "clear-orbits", Size::Grow, button_height)
            .enabled(!state.queued_orbits.is_empty()),
    );

    sidebar.add_child(
        Node::button("Create Group", "create-group", Size::Grow, button_height)
            .enabled(!state.track_list.is_empty()),
    );

    sidebar.add_child(Node::hline());

    sidebar.add_child({
        let s = format!("{} selected", state.track_list.len());
        Node::button(s, "", Size::Grow, button_height).enabled(false)
    });

    if state.track_list.len() <= 12 {
        for id in &state.track_list {
            let s = format!("Object {}", id);
            let id = format!("object-{}", id);
            sidebar.add_child(Node::button(s, id, Size::Grow, button_height));
        }
    }

    if let Some(fid) = state.follow {
        if state.track_list.contains(&fid) {
            sidebar.add_child(Node::hline());
            let s = format!("Pilot {}", fid);
            let id = "manual-control";
            sidebar.add_child(Node::button(s, id, Size::Grow, button_height));
        }
    }

    if !state.controllers.is_empty() {
        sidebar.add_child(Node::hline());
        let s = format!("{} autopiloting", state.controllers.len());
        sidebar.add_child(Node::button(s, "", Size::Grow, button_height).enabled(false));
    }

    if !state.constellations.is_empty() {
        sidebar.add_child(Node::hline());
    }

    for (gid, members) in &state.constellations {
        let s = format!("GID {} ({})", gid, members.len());
        let id = format!("gid-{}", gid);
        sidebar.add_child(Node::button(s, id, Size::Grow, button_height));
    }

    let mut world = Node::grow()
        .invisible()
        .with_id("world")
        .with_child({
            let s = if state.paused { "UNPAUSE" } else { "PAUSE" };
            Node::button(s, "toggle-pause", 120, button_height)
        })
        .with_children(
            (0..6).map(|i| Node::button(format!("{i}"), "", button_height, button_height)),
        );

    for orbit in &state.queued_orbits {
        let s = format!("{}", orbit);
        world.add_child(Node::button(s, "", 400, button_height));
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

    if let Some((_, c)) = state.mouse.right().zip(state.mouse.current()) {
        let ctx = context_menu((c.x / 30.0) as u32 % 10, (c.y / 30.0) as u32 % 10);
        let c = Vec2::new(c.x, state.camera.viewport_bounds().span.y - c.y);
        tree.add_layout(ctx, c);
    }

    Some(tree)
}

#[derive(Component)]
struct UiElement;

fn do_ui_sprites(
    mut commands: Commands,
    to_despawn: Query<Entity, With<UiElement>>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<GameState>,
) {
    if state.actual_time - state.last_redraw < Nanotime::millis(250) {
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

    for layout in state.ui.layouts() {
        let mut nodes: Vec<_> = layout.iter(0).collect();

        nodes.sort_by_key(|(l, _)| *l);

        for (layer, n) in nodes {
            if !n.is_visible() {
                continue;
            }

            let aabb = n.aabb();
            let w = (aabb.span.x as u32).max(1);
            let h = (aabb.span.y as u32).max(1);

            let color = if n.is_leaf() && n.is_enabled() {
                GRAY.with_luminance(0.5).with_alpha(0.8)
            } else if n.is_leaf() {
                GRAY.with_luminance(0.2).with_alpha(0.8)
            } else {
                GRAY.with_luminance(0.2).with_alpha(0.1)
            };

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

            if w != 1 && h != 1 && n.is_leaf() {
                for y in 0..h {
                    for x in 0..w {
                        if x < 3 || y < 3 || x > w - 4 || y > h - 4 {
                            if let Some(bytes) = image.pixel_bytes_mut(UVec3::new(x, y, 0)) {
                                bytes[3] = 80;
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

            let mut c = aabb.center;

            c.x -= vb.span.x / 2.0;
            c.y = vb.span.y / 2.0 - c.y;

            let transform = Transform::from_translation(c.extend(layer as f32 / 10.0));

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
