use crate::game::GameState;
use crate::input::{FrameId, MouseButt};
use crate::onclick::OnClick;
use crate::scenes::*;
use bevy::core_pipeline::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    view::RenderLayers,
};
use bevy::sprite::Anchor;
use bevy::text::TextBounds;
use bevy::window::WindowResized;
use layout::layout::{Node, Size, TextJustify, Tree};
use starling::prelude::*;

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
    SetSim(i32),
    ClearSelection,
    ClearOrbitQueue,
    Escape,
    Save,
    Restore,
    Load(String),
    ToggleObject(OrbiterId),
    ToggleGroup(GroupId),
    DisbandGroup(GroupId),
    CreateGroup,
    ContextDependent,
    CursorMode,
    DrawMode,
    RedrawGui,
    ToggleFullscreen,

    // orbital_context operations
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    ZoomIn,
    ZoomOut,
    Reset,

    // manual piloting commands
    Thrust(i8),
    TurnLeft,
    TurnRight,
    StrafeLeft,
    StrafeRight,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, (do_ui_sprites, set_bloom, on_resize_system));
    }
}

fn set_bloom(state: Res<GameState>, mut bloom: Single<&mut Bloom>) {
    bloom.intensity = match state.current_scene().kind() {
        SceneType::MainMenu => 0.6,
        SceneType::Orbital => match state.orbital_context.draw_mode {
            DrawMode::Default => 0.5,
            _ => 0.1,
        },
        _ => 0.1,
    }
}

const TEXT_LABEL_Z_INDEX: f32 = 100.0;

pub fn do_text_labels(
    mut commands: Commands,
    state: Res<GameState>,
    mut query: Query<
        (
            Entity,
            &mut Text2d,
            &mut TextFont,
            &mut Transform,
            &mut TextColor,
            &mut Anchor,
        ),
        With<TextLabel>,
    >,
) {
    let text_labels = GameState::text_labels(&state).unwrap_or(vec![]);

    let mut labels: Vec<_> = query.iter_mut().collect();
    for (i, tl) in text_labels.iter().enumerate() {
        if let Some((_, text2d, font, label, color, anchor)) = labels.get_mut(i) {
            label.translation = tl.position.extend(TEXT_LABEL_Z_INDEX);
            label.scale = Vec3::splat(1.0);
            text2d.0 = tl.text.clone();
            font.font_size = 23.0 * tl.size;
            color.0 = tl.color().into();
            **anchor = tl.anchor;
        } else {
            commands.spawn((
                Text2d::new(tl.text.clone()),
                TextFont {
                    font_size: 23.0 * tl.size,
                    ..default()
                },
                Transform::from_translation(tl.position.extend(TEXT_LABEL_Z_INDEX)),
                TextLabel,
                TextColor(tl.color().into()),
                tl.anchor,
            ));
        }
    }

    for (i, (e, _, _, _, _, _)) in query.iter().enumerate() {
        if i >= text_labels.len() {
            commands.entity(e).despawn();
        }
    }
}

#[derive(Component)]
pub struct TextLabel;

#[allow(unused)]
fn context_menu(rowsize: f32, items: &[(String, OnClick, bool)]) -> Node<OnClick> {
    Node::new(200, Size::Fit)
        .down()
        .with_color([0.1, 0.1, 0.1, 1.0])
        .with_children(items.iter().map(|(s, id, e)| {
            Node::button(s, id.clone(), Size::Grow, rowsize)
                .with_color([0.3, 0.3, 0.3, 1.0])
                .enabled(*e)
        }))
}

pub const DELETE_SOMETHING_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
pub const UI_BACKGROUND_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
pub const PILOT_FAVORITES_COLOR: [f32; 4] = [0.8, 0.8, 0.2, 1.0];

pub fn top_bar(state: &GameState) -> Node<OnClick> {
    Node::row(Size::Fit)
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(Node::button("Save", OnClick::Save, 80, Size::Grow))
        .with_child(Node::button("Load", OnClick::Load, 80, Size::Grow))
        .with_child(Node::vline())
        .with_children(state.scenes.iter().enumerate().map(|(i, scene)| {
            let s = scene.name();
            let id = OnClick::GoToScene(i);
            let current = state.current_scene_idx == i;
            Node::button(s, id, 120, BUTTON_HEIGHT).enabled(!current)
        }))
        .with_child(Node::grow().invisible())
        .with_child(Node::button("Exit", OnClick::Exit, 80, Size::Grow))
}

pub fn basic_scenes_layout(state: &GameState) -> Tree<OnClick> {
    let vb = state.input.screen_bounds;
    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return Tree::new();
    }

    let top_bar = top_bar(state);
    let notif_bar = notification_bar(state, Size::Fixed(900.0));

    let layout = Node::new(vb.span.x, vb.span.y)
        .tight()
        .invisible()
        .down()
        .with_child(top_bar)
        .with_child(notif_bar);

    Tree::new().with_layout(layout, Vec2::ZERO)
}

pub fn notification_bar(state: &GameState, width: Size) -> Node<OnClick> {
    Node::new(width, Size::Fit)
        .down()
        .tight()
        .invisible()
        .with_children(state.notifications.iter().rev().take(20).rev().map(|n| {
            let s = format!("{}", n);
            Node::new(width, 28)
                .with_text(s)
                .with_justify(TextJustify::Left)
                .with_color([0.0, 0.0, 0.0, 0.0])
        }))
}

pub const BUTTON_HEIGHT: f32 = 36.0;

pub fn exit_prompt_overlay(w: f32, h: f32) -> Node<OnClick> {
    let window = Node::new(330, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(Node::row(BUTTON_HEIGHT).with_text("Exit?").enabled(false))
        .with_child(Node::button(
            "Yes Sir",
            OnClick::ConfirmExitDialog,
            Size::Grow,
            BUTTON_HEIGHT,
        ))
        .with_child(Node::button(
            "No Way",
            OnClick::DismissExitDialog,
            Size::Grow,
            BUTTON_HEIGHT,
        ));

    let col = Node::column(Size::Fit)
        .invisible()
        .down()
        .with_child(Node::grow().invisible())
        .with_child(window)
        .with_child(Node::grow().invisible());

    Node::new(w, h)
        .with_color([0.0, 0.0, 0.0, 0.95])
        .with_child(Node::grow().invisible())
        .with_child(col)
        .with_child(Node::grow().invisible())
}

pub fn delete_wrapper(ondelete: OnClick, button: Node<OnClick>, box_size: f32) -> Node<OnClick> {
    let x_button = {
        let s = "X";
        Node::button(s, ondelete, box_size, box_size).with_color(DELETE_SOMETHING_COLOR)
    };

    let (w, _) = button.desired_dims();

    let width = match w {
        Size::Fit => Size::Fit,
        Size::Fixed(n) => Size::Fixed(n + box_size),
        Size::Grow => Size::Grow,
    };

    Node::new(width, box_size)
        .tight()
        .invisible()
        .with_child(x_button)
        .with_child(button)
}

pub fn piloting_buttons(state: &GameState, width: Size) -> Node<OnClick> {
    let mut wrapper = Node::new(width, Size::Fit)
        .down()
        .invisible()
        .with_padding(0.0);

    let _x = if let Some(p) = state.orbital_context.piloting {
        wrapper.add_child({
            let s = format!("Piloting {:?}", p);
            let b = Node::button(s, OnClick::Orbiter(p), Size::Grow, BUTTON_HEIGHT);
            delete_wrapper(OnClick::ClearPilot, b, BUTTON_HEIGHT as f32)
        });
    } else if let Some(ObjectId::Orbiter(p)) = state.orbital_context.following {
        wrapper.add_child({
            let s = format!("Pilot {:?}", p);
            Node::button(s, OnClick::SetPilot(p), Size::Grow, BUTTON_HEIGHT)
        });
    } else {
        wrapper.add_child(
            Node::button(
                "No craft selected",
                OnClick::Nullopt,
                Size::Grow,
                BUTTON_HEIGHT,
            )
            .enabled(false),
        );
    };

    let _y = if let Some(p) = state.orbital_context.targeting {
        wrapper.add_child({
            let s = format!("Targeting {:?}", p);
            let b = Node::button(s, OnClick::Orbiter(p), Size::Grow, BUTTON_HEIGHT);
            delete_wrapper(OnClick::ClearTarget, b, BUTTON_HEIGHT as f32)
        });
        true
    } else if let Some(ObjectId::Orbiter(p)) = state.orbital_context.following {
        wrapper.add_child({
            let s = format!("Target {:?}", p);
            Node::button(s, OnClick::SetTarget(p), Size::Grow, BUTTON_HEIGHT)
        });
        true
    } else {
        false
    };

    if state.piloting().is_some() && state.targeting().is_some() {
        wrapper.add_child({
            Node::button(
                "Swap",
                OnClick::SwapOwnshipTarget,
                Size::Grow,
                BUTTON_HEIGHT,
            )
        });
    }

    wrapper
}

pub fn selected_button(state: &GameState, width: Size) -> Node<OnClick> {
    let s = format!("{} selected", state.orbital_context.selected.len());
    let b = Node::button(s, OnClick::SelectedCount, width, BUTTON_HEIGHT).enabled(false);
    if state.orbital_context.selected.is_empty() {
        b
    } else {
        delete_wrapper(OnClick::ClearTracks, b, BUTTON_HEIGHT as f32)
    }
}

pub fn orbiter_list(
    state: &GameState,
    root: &mut Node<OnClick>,
    max_cells: usize,
    mut ids: Vec<OrbiterId>,
) {
    ids.sort();

    let rows = (ids.len().min(max_cells) as f32 / 4.0).ceil() as u32;
    let grid = Node::grid(Size::Grow, rows * BUTTON_HEIGHT as u32, rows, 4, 4.0, |i| {
        if i as usize > max_cells {
            return None;
        }
        let id = ids.get(i as usize)?;
        let s = format!("{id}");
        Some(
            Node::grow()
                .with_on_click(OnClick::Orbiter(*id))
                .with_text(s)
                .enabled(
                    Some(*id)
                        != state
                            .orbital_context
                            .following
                            .map(|f| f.orbiter())
                            .flatten(),
                ),
        )
    });
    root.add_child(grid);

    if ids.len() > max_cells {
        let n = ids.len() - max_cells;
        let s = format!("...And {} more", n);
        root.add_child(
            Node::new(Size::Grow, BUTTON_HEIGHT)
                .with_text(s)
                .enabled(false),
        );
    }
}

pub fn left_right_arrows(
    width: impl Into<Size>,
    height: impl Into<Size>,
    left: OnClick,
    right: OnClick,
) -> Node<OnClick> {
    let height = height.into();
    let left = Node::button("-", left, Size::Grow, height);
    let right = Node::button("+", right, Size::Grow, height);
    Node::new(width, height)
        .with_padding(0.0)
        .invisible()
        .with_child(left)
        .with_child(right)
}

pub fn favorites_menu(state: &GameState) -> Node<OnClick> {
    let mut wrapper = Node::structural(350, Size::Fit)
        .down()
        .with_child(Node::text(Size::Grow, BUTTON_HEIGHT, "Favorites").enabled(false))
        .with_color(UI_BACKGROUND_COLOR)
        .with_children({
            state.favorites.iter().filter_map(|id| {
                let name = state.vehicles.get(id).map(|v| v.name()).unwrap_or(&"?");
                let s = format!("{} {}", name, id);
                let b = Node::button(s, OnClick::Orbiter(*id), Size::Grow, BUTTON_HEIGHT);
                let d = delete_wrapper(OnClick::RemoveFromFavorites(*id), b, BUTTON_HEIGHT);
                let p = Node::button("", OnClick::SetPilot(*id), BUTTON_HEIGHT, BUTTON_HEIGHT);
                Some(d.with_child(p.with_color(PILOT_FAVORITES_COLOR)))
            })
        });

    if let Some(ObjectId::Orbiter(id)) = state.orbital_context.following {
        wrapper.add_child(Node::hline());
        let s = format!("Add {}", id);
        let b = Node::button(s, OnClick::AddToFavorites(id), Size::Grow, BUTTON_HEIGHT)
            .enabled(!state.favorites.contains(&id));
        wrapper.add_child(b);
    }

    wrapper
}

pub fn throttle_controls(state: &GameState) -> Node<OnClick> {
    const THROTTLE_CONTROLS_WIDTH: f32 = 300.0;

    if !state.piloting().is_some() {
        return Node::new(0.0, 0.0);
    }

    let arrows = left_right_arrows(
        Size::Grow,
        BUTTON_HEIGHT,
        OnClick::IncrementThrottle(-1),
        OnClick::IncrementThrottle(1),
    );

    let throttle = state.orbital_context.throttle;

    let title = format!(
        "Throttle ({}%)",
        (throttle.to_ratio() * 100.0).round() as i32
    );

    Node::new(THROTTLE_CONTROLS_WIDTH, Size::Fit)
        .with_color(UI_BACKGROUND_COLOR)
        .down()
        .with_child(Node::row(BUTTON_HEIGHT).with_text(title).enabled(false))
        .with_child(
            Node::row(BUTTON_HEIGHT)
                .invisible()
                .with_padding(0.0)
                .with_child_gap(2.0)
                .with_children((0..=ThrottleLevel::MAX).map(|i| {
                    let t = ThrottleLevel(i);
                    let onclick = OnClick::ThrottleLevel(t);
                    let n =
                        Node::button("", onclick, Size::Grow, BUTTON_HEIGHT).enabled(t != throttle);
                    if i < throttle.0 {
                        n.with_color([0.8, 0.2, 0.2, 0.9])
                    } else {
                        n.with_color([0.9, 0.9, 0.9, 0.7])
                    }
                })),
        )
        .with_child(arrows)
}

pub fn sim_time_toolbar(state: &GameState) -> Node<OnClick> {
    use crate::game::{MAX_SIM_SPEED, MIN_SIM_SPEED};
    Node::fit()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child({
            let s = if state.paused { "UNPAUSE" } else { "PAUSE" };
            Node::button(s, OnClick::TogglePause, 120, BUTTON_HEIGHT)
        })
        .with_children((MIN_SIM_SPEED..=MAX_SIM_SPEED).map(|i| {
            Node::button(
                format!("{i}"),
                OnClick::SimSpeed(i),
                BUTTON_HEIGHT,
                BUTTON_HEIGHT,
            )
            .enabled(i != state.sim_speed)
        }))
}

pub fn layout(state: &GameState) -> Tree<OnClick> {
    let scene = state.current_scene();
    match scene.kind() {
        SceneType::MainMenu => MainMenuContext::ui(state),
        SceneType::DockingView => RPOContext::ui(state),
        SceneType::Telescope => TelescopeContext::ui(state),
        SceneType::Orbital => OrbitalContext::ui(state),
        SceneType::Editor => EditorContext::ui(state),
        SceneType::CommsPanel => CommsContext::ui(state),
        SceneType::Surface => SurfaceContext::ui(state),
    }
    .unwrap_or(Tree::new())
}

#[allow(unused)]
fn current_inventory_layout(state: &GameState) -> Option<Node<OnClick>> {
    let id = state.orbital_context.following?.orbiter()?;
    let orbiter = state.lup_orbiter(id, state.sim_time)?.orbiter()?;
    let vehicle = state.vehicles.get(&id)?;

    if vehicle.inventory.is_empty() {
        return None;
    }

    let buttons = Node::new(Size::Grow, Size::Fit)
        .down()
        .with_child({
            let s = format!("Vehicle {}", vehicle.name());
            Node::button(s, OnClick::Nullopt, Size::Grow, 40.0).enabled(false)
        })
        .with_children(vehicle.inventory.iter().map(|(k, v)| {
            let name = format!("{:?} {} g", k, v);
            Node::button(name, OnClick::Nullopt, Size::Grow, 40.0)
        }));

    Some(
        // TODO this node should be fit
        Node::new(400.0, Size::Fit)
            .tight()
            .down()
            .with_child(Node::new(Size::Grow, 30.0).with_color([0.2, 0.2, 0.2, 0.9]))
            .with_child(buttons),
    )
}

#[derive(Component)]
struct UiElement;

fn map_bytes(image: &mut Image, func: impl Fn(&mut [u8], u32, u32, u32, u32)) {
    let w = image.width();
    let h = image.height();
    for x in 0..w {
        for y in 0..h {
            if let Some(bytes) = image.pixel_bytes_mut(UVec3::new(x, y, 0)) {
                func(bytes, x, y, w, h);
            }
        }
    }
}

fn generate_button_sprite(
    node: &Node<OnClick>,
    is_clicked: bool,
    is_hover: bool,
) -> (Image, f32, f32) {
    let aabb = node.aabb();
    let w = (aabb.span.x as u32).max(1);
    let h = (aabb.span.y as u32).max(1);

    let color = node.color();
    let color = Srgba::new(color[0], color[1], color[2], color[3]);

    let get_image = |w: u32, h: u32| {
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
        image
    };

    if !node.is_leaf() || w == 1 || h == 1 || !node.is_enabled() {
        return (get_image(1, 1), aabb.span.x, aabb.span.y);
    }

    let mut image = get_image(w, h);

    if is_hover {
        map_bytes(&mut image, |bytes, _, _, _, _| {
            for i in 0..3 {
                let b = bytes[i] as f32;
                bytes[i] = (b * 0.8) as u8;
            }
        });
    }

    if is_clicked {
        map_bytes(&mut image, |bytes, x, y, _, _| {
            if x < 2 || y < 2 || x + 2 >= w || y + 2 >= h {
                bytes[3] = 0;
            } else {
                for i in 0..3 {
                    let b = bytes[i] as f32;
                    bytes[i] = (b * 0.6) as u8;
                }
            }
        });
    }

    (image, 1.0, 1.0)
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

    if ui_age < Nanotime::millis(250) && !state.redraw_requested {
        return;
    }

    let vb = state.input.screen_bounds;

    for e in &to_despawn {
        commands.entity(e).despawn();
    }

    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return;
    }

    let mut ui = layout(&state);

    if state.is_exit_prompt {
        ui.add_layout(exit_prompt_overlay(vb.span.x, vb.span.y), Vec2::ZERO)
    }

    state.ui = ui;

    state.last_redraw = state.wall_time;
    state.redraw_requested = false;

    for (lid, layout) in state.ui.layouts().iter().enumerate() {
        for n in layout.iter() {
            if !n.is_visible() {
                continue;
            }

            let aabb = n.aabb_camera(vb.span);
            let hover = state.input.position(MouseButt::Hover, FrameId::Current);
            let left = state.input.position(MouseButt::Left, FrameId::Current);
            let left_down = state.input.position(MouseButt::Left, FrameId::Down);
            let is_hover = hover.map(|p| aabb.contains(p)).unwrap_or(false);
            let is_clicked = left.map(|p| aabb.contains(p)).unwrap_or(false)
                && left_down.map(|p| aabb.contains(p)).unwrap_or(false);
            let (image, sx, sy) = generate_button_sprite(n, is_clicked, is_hover);

            let c = aabb.center;

            let transform =
                Transform::from_translation(c.extend(n.layer() as f32 / 100.0 + lid as f32));

            let handle = images.add(image);

            commands.spawn((
                transform.with_scale(Vec3::new(sx, sy, 1.0)),
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
                    transform.translation.x += match n.justify() {
                        TextJustify::Center => 0.0,
                        TextJustify::Left => -aabb.span.x / 2.0,
                        TextJustify::Right => aabb.span.x / 2.0,
                    };

                    let anchor = match n.justify() {
                        TextJustify::Center => Anchor::Center,
                        TextJustify::Left => Anchor::CenterLeft,
                        TextJustify::Right => Anchor::CenterRight,
                    };

                    commands.spawn((
                        transform,
                        bounds,
                        Text2d::new(s.to_uppercase()),
                        anchor,
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
}

fn on_resize_system(mut resize_reader: EventReader<WindowResized>, mut state: ResMut<GameState>) {
    for _ in resize_reader.read() {
        state.redraw();
    }
}
