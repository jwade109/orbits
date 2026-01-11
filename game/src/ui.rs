use crate::game::GameState;
use crate::starling::prelude::*;
use bevy::prelude::*;
use bevy::sprite::Anchor;

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
