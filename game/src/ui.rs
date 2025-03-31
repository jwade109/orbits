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

pub fn layout(state: &GameState) -> Option<ui::Tree> {
    use ui::*;

    let vb = state.camera.viewport_bounds();
    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return None;
    }

    let buttons = [
        ("Commit Mission", "commit-mission"),
        ("Clear Orbits", "clear-orbits"),
    ];

    let topbar = Node::row(70).with_children((0..5).map(|_| Node::column(120)));

    let sidebar = Node::column(300).with_children(
        buttons
            .iter()
            .map(|(s, id)| Node::button(s, id, Size::Grow, 60)),
    );

    let root = Node::new(vb.span.x, vb.span.y)
        .down()
        .tight()
        .invisible()
        .with_child(topbar)
        .with_child(Node::grow().with_child(sidebar));

    let tree = Tree::new().with_layout(root, Vec2::ZERO);

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
    if !state.redraw_gui {
        return;
    }

    let vb = state.camera.viewport_bounds();

    for e in &to_despawn {
        commands.entity(e).despawn();
    }

    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return;
    }

    println!("Redrawing GUI {:?}", vb);

    let ui = layout::examples::example_layout(vb.span.x, vb.span.y);

    // let ui = match layout(&state) {
    //     Some(ui) => ui,
    //     None => return,
    // };

    state.redraw_gui = false;

    for layout in ui.layouts() {
        let mut nodes = layout.visit(
            &|layer, n: &layout::layout::Node| n.is_visible().then(move || (layer, n.clone())),
            0,
        );

        nodes.sort_by_key(|(l, _)| *l);

        for (layer, n) in nodes {
            let aabb = n.aabb();
            let w = (aabb.span.x as u32).max(1);
            let h = (aabb.span.y as u32).max(1);

            let color = if n.is_leaf() {
                GRAY.with_luminance(0.5).with_alpha(0.6)
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
                        if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
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
