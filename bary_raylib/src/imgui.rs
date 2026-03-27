use crate::client::ClientSpecificInfo;
use crate::render::draw::*;
use crate::sim::World;
use bary_core::prelude::*;
use early_returns::*;
use raylib::prelude::*;

pub fn imgui_all_parts_in_layer(
    d: &mut RaylibDrawHandle,
    client: &mut ClientSpecificInfo,
    world: &World,
) {
    let hovered_proto = get_hovered_prototype(client, world);
    let editor = some_or_return!(client.viewport.editor_mut());
    let layer = some_or_return!(editor.layer);
    let mouse_pos = client.mouse_screen_position.unwrap_or(Vec2::NAN);

    let height = d.get_render_height();

    let bottom_left = IVec2::new(50, height - 50);
    let font_size = 18;
    let box_width = 200;
    let box_height = 40;
    let padding = 2;

    let mut y = bottom_left.y;

    for (proto_id, proto) in world.prototypes.iter().rev() {
        if proto.layer != layer {
            continue;
        }
        let is_hovered = Some(*proto_id) == hovered_proto;
        let is_selected = Some(*proto_id) == editor.prototype_id;
        y -= box_height + padding;
        let xc = bottom_left.x as f32 + box_width as f32 / 2.0;
        let yc = y as f32 + box_height as f32 / 2.0;
        let center = Vec2::new(xc, yc);
        let alpha = 0.96;

        let color = if is_selected {
            Color::ORANGE
        } else if is_hovered {
            Color::YELLOW
        } else {
            Color::DARKSLATEGRAY
        };

        let aabb = AABB::from_wh(box_width as f32, box_height as f32).with_center(center);

        let xc = bottom_left.x as f32 + box_width as f32 / 2.0;
        let yc = y as f32 + box_height as f32 / 4.0;
        let center = Vector2::new(xc, yc);
        d.draw_rectangle(bottom_left.x, y, box_width, box_height, color.alpha(alpha));
        if aabb.contains(mouse_pos) {
            d.draw_rectangle_lines(bottom_left.x, y, box_width, box_height, Color::WHITE);
            editor.prototype_id = Some(*proto_id);
        }
        draw_text_centered(d, &proto.name, center, font_size);
    }
}

pub fn imgui_editor_layer_indicator(d: &mut RaylibDrawHandle, client: &mut ClientSpecificInfo) {
    let editor = some_or_return!(client.viewport.editor_mut());
    let mouse_pos = client.mouse_screen_position.unwrap_or(Vec2::NAN);

    let boxes = [
        (PartLayer::Exterior, Color::WHITE),
        (PartLayer::Structural, Color::GRAY),
        (PartLayer::Plumbing, Color::PURPLE),
        (PartLayer::Internal, Color::ORANGE),
    ];

    let width = d.get_render_width();
    let height = d.get_render_height();

    let font_size = 18;
    let box_width = 200;
    let box_height = 40;
    let padding = 2;
    let bottom_right = IVec2::new(width - 50, height - 50);
    let dims = IVec2::new(
        box_width,
        boxes.len() as i32 * box_height + padding * (boxes.len() as i32 - 1),
    );
    let origin = bottom_right - dims;
    let bottom_left = bottom_right - IVec2::X * dims.x;

    let mut y = origin.y;

    for (layer, color) in boxes {
        let xc = origin.x as f32 + box_width as f32 / 2.0;
        let yc = y as f32 + box_height as f32 / 2.0;
        let center = Vec2::new(xc, yc);
        let aabb = AABB::from_wh(box_width as f32, box_height as f32).with_center(center);

        let xc = origin.x as f32 + box_width as f32 / 2.0;
        let yc = y as f32 + box_height as f32 / 4.0;
        let center = Vector2::new(xc, yc);
        let text = format!("{:?}", layer);
        let is_focused = Some(layer) == editor.layer || editor.layer.is_none();
        let alpha = if is_focused { 1.0 } else { 0.2 };
        d.draw_rectangle(origin.x, y, box_width, box_height, color.alpha(alpha));
        draw_text_centered(d, &text, center, font_size);

        if aabb.contains(mouse_pos) {
            d.draw_rectangle_lines(bottom_left.x, y, box_width, box_height, Color::WHITE);
            editor.layer = Some(layer);
        }

        y += box_height + padding;
    }
}

pub fn imgui_entrypoint(
    d: &mut RaylibDrawHandle,
    world: &mut World,
    client: &mut ClientSpecificInfo,
) {
    imgui_editor_layer_indicator(d, client);
    imgui_all_parts_in_layer(d, client, world);
}
