use bary_core::prelude::*;
use raylib::prelude::*;

pub fn draw_window(d: &mut RaylibDrawHandle, title: &str, text: &str, origin: IVec2, font: &Font) {
    let font_size = 23;
    let spacing = 1.0;
    let padding = 7;
    let child_gap = 7;

    let title_dims = font.measure_text(&title, font_size as f32, spacing);
    let title_dims = IVec2::new(title_dims.x as i32, title_dims.y as i32);
    let header_height: i32 = title_dims.y + padding * 2;

    let title_text_origin = origin + IVec2::splat(padding);

    let text_dims = font.measure_text(text, font_size as f32, spacing);
    let text_dims = IVec2::new(text_dims.x as i32, text_dims.y as i32);
    let content_dims = text_dims + IVec2::splat(2 * padding);
    let window_dims =
        content_dims + IVec2::new(2 * padding, child_gap + 2 * padding + header_height);

    let content_origin = origin + IVec2::new(padding, padding + header_height + child_gap);
    let text_origin = content_origin + IVec2::splat(padding);

    // whole window
    d.draw_rectangle(
        origin.x,
        origin.y,
        window_dims.x,
        window_dims.y,
        Color::GRAY,
    );

    // window header
    d.draw_rectangle(
        origin.x,
        origin.y,
        window_dims.x,
        header_height,
        Color::ORANGE,
    );

    // content area
    d.draw_rectangle(
        content_origin.x,
        content_origin.y,
        content_dims.x,
        content_dims.y,
        Color::new(40, 40, 40, 255),
    );

    let position = Vector2::new(text_origin.x as f32, text_origin.y as f32);
    d.set_text_line_spacing(0);

    d.draw_text_ex(
        font,
        title,
        Vector2::new(title_text_origin.x as f32, title_text_origin.y as f32),
        font_size as f32,
        spacing,
        Color::BLACK,
    );

    d.draw_text_ex(
        font,
        &text,
        position,
        font_size as f32,
        spacing,
        Color::WHITE,
    );
}
