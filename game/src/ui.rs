use crate::game::GameState;
use crate::input::{FrameId, MouseButt};
use crate::layout::layout::{Node, Size, TextJustify, Tree};
use crate::onclick::OnClick;
use crate::scenes::*;
use crate::starling::prelude::*;
use bevy::core_pipeline::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    view::RenderLayers,
};
use bevy::sprite::Anchor;
use bevy::text::TextBounds;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (do_ui_sprites, set_bloom));
    }
}

fn set_bloom(state: Res<GameState>, mut bloom: Single<&mut Bloom>) {
    bloom.intensity = 0.1;
}

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
    asset_server: Res<AssetServer>,
) {
    let text_labels = state.text_labels.clone();

    let font_ttf = "Lato-Bold.ttf";

    let font = match std::fs::canonicalize(state.args.fonts_dir().join(font_ttf)) {
        Ok(path) => asset_server.load(path),
        Err(e) => {
            error!("Failed to play sound: {}", e);
            return;
        }
    };

    let font_size = 28.0;

    let mut labels: Vec<_> = query.iter_mut().collect();
    for (i, tl) in text_labels.iter().enumerate() {
        if let Some((_, text2d, font, label, color, anchor)) = labels.get_mut(i) {
            label.translation = tl.pos().extend(tl.z_order().as_f32());
            label.scale = Vec3::splat(tl.size());
            text2d.0 = tl.text().to_string();
            font.font_size = font_size;
            color.0 = tl.color().into();
            **anchor = tl.anchor();
        } else {
            commands.spawn((
                Text2d::new(tl.text()),
                TextFont::from_font_size(font_size).with_font(font.clone()),
                Transform::from_translation(tl.pos().extend(tl.z_order().as_f32()))
                    .with_scale(Vec3::splat(tl.size())),
                TextLabel,
                TextColor(tl.color().into()),
                tl.anchor(),
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

pub const DELETE_SOMETHING_COLOR: [f32; 4] = [1.0, 0.3, 0.3, 1.0];
pub const UI_BACKGROUND_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
pub const PILOT_FAVORITES_COLOR: [f32; 4] = [0.3, 0.3, 0.9, 1.0];
pub const EXIT_OVERLAY_BACKGROUND_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.95];

pub fn top_bar(state: &GameState) -> Node<OnClick> {
    Node::row(Size::Fit)
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(Node::button("Save", OnClick::Save, 80, Size::Grow))
        .with_child(Node::button("Load", OnClick::Load, 80, Size::Grow))
        .with_child(Node::vline())
        .with_child(Node::button("Exit", OnClick::Exit, 80, Size::Grow))
}

pub fn basic_scenes_layout(state: &GameState) -> Tree<OnClick> {
    let vb = state.input.screen_bounds;
    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return Tree::new();
    }

    let top_bar = top_bar(state);

    let layout = Node::new(vb.span.x, vb.span.y)
        .tight()
        .invisible()
        .down()
        .with_child(top_bar);

    Tree::new().with_layout(layout, Vec2::ZERO)
}

#[deprecated]
pub const BUTTON_HEIGHT: f32 = 29.0;

pub fn exit_prompt_overlay(button_height: f32, w: f32, h: f32) -> Node<OnClick> {
    let window = Node::new(330, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(Node::row(button_height).with_text("Exit?").enabled(false))
        .with_child(Node::button(
            "Yes Sir",
            OnClick::ConfirmExitDialog,
            Size::Grow,
            button_height,
        ))
        .with_child(Node::button(
            "No Way",
            OnClick::DismissExitDialog,
            Size::Grow,
            button_height,
        ));

    let col = Node::column(Size::Fit)
        .invisible()
        .down()
        .with_child(Node::grow().invisible())
        .with_child(window)
        .with_child(Node::grow().invisible());

    Node::new(w, h)
        .with_color(EXIT_OVERLAY_BACKGROUND_COLOR)
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

pub fn layout(state: &GameState) -> Tree<OnClick> {
    Tree::new()
    // Editor::ui(state).unwrap_or(Tree::new())
}

pub fn apply_egui_style(ui: &mut egui::Ui) {
    let x = ui.style_mut();
    x.spacing.window_margin = egui::Margin::same(40);
    x.spacing.item_spacing.y = 5.0;
    x.spacing.button_padding.x = 5.0;
    x.spacing.button_padding.y = 5.0;
    x.visuals.dark_mode = false;
    for x in &mut x.text_styles {
        x.1.size *= 1.2;
    }
}

#[derive(Component)]
pub struct UiElement;

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
    let vb = state.input.screen_bounds;

    for e in &to_despawn {
        commands.entity(e).despawn();
    }

    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return;
    }

    let mut ui = layout(&state);

    if state.is_exit_prompt {
        ui.add_layout(
            exit_prompt_overlay(state.settings.ui_button_height, vb.span.x, vb.span.y),
            Vec2::ZERO,
        )
    }

    state.ui = ui;

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

            if let Some(sprite) = n.sprite() {
                if let Some((handle, dims)) = state.image_handles.get(sprite) {
                    let mut transform = transform;
                    transform.translation.z += 0.01;
                    let sx = aabb.span.x / dims.x as f32;
                    let sy = aabb.span.y / dims.y as f32;
                    let s = sx.min(sy);
                    commands.spawn((
                        transform.with_scale(Vec3::new(s, s, 1.0)),
                        Sprite::from_image(handle.clone()),
                        RenderLayers::layer(1),
                        UiElement,
                    ));
                }
            }

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
                        Text2d::new(s),
                        anchor,
                        RenderLayers::layer(1),
                        UiElement,
                    ));
                }
            }
        }
    }
}
