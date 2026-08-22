use crate::bezier::BezierCurve;
use crate::event_bus::EventBus;
use crate::track::TrackSegment;
use crate::tweens::AnimationStates;
use crate::viewport::Viewport;
use crate::world::*;
use bary_core::prelude::*;
use bary_input::InputState;
use rend::*;

fn draw_button(
    cmd: &mut RenderCommands,
    text: &str,
    p: DVec2,
    mouse: DVec2,
    input: &InputState,
) -> (DVec2, bool) {
    let padding = DVec2::splat(15.0);
    let (tcmd, extent) = cmd.text(p + padding, text, 32.0, Color::WHITE);
    let full_extent = extent + padding * 2.0;
    let aabb = AABB::from_arbitrary(p.as_vec2(), (p + full_extent).as_vec2());
    let contains = aabb.contains(mouse.as_vec2());
    let alpha = contains as u8 as f64 * 0.2 + 0.9;
    cmd.rect(p)
        .dims(full_extent)
        .color(Color::BLUE.alpha(alpha));
    cmd.apply(tcmd);
    (
        full_extent,
        input.just_pressed(rdev::Button::Left) && contains,
    )
}

fn draw_grid_lines(cmd: &mut RenderCommands, view: &Viewport) {
    let n_lines = 500;
    let spacing = 25;

    for x in -n_lines..=n_lines {
        let x = x * spacing;
        let s = DVec2::new(x as f64, -10000.0);
        let e = DVec2::new(x as f64, 10000.0);
        cmd.line(view.world_to_screen(s), view.world_to_screen(e))
            .color(Color::GRAY)
            .thickness(3.0);
        let s = DVec2::new(-10000.0, x as f64);
        let e = DVec2::new(10000.0, x as f64);
        cmd.line(view.world_to_screen(s), view.world_to_screen(e))
            .color(Color::GRAY)
            .thickness(3.0);
    }
}

fn draw_font_ui(cmd: &mut RenderCommands, events: &mut EventBus, mouse: DVec2, input: &InputState) {
    let fonts = cmd.fonts.clone();

    let mut p = DVec2::new(30.0, 300.0);
    for (font_id, font) in fonts {
        let text = format!("{} {}", font_id, font.name);
        let (e, clicked) = draw_button(cmd, &text, p, mouse, input);
        if clicked {
            events.clicked(font_id);
        }
        p.y += e.y + 15.0;
    }
}

fn draw_bezier(cmd: &mut RenderCommands, bezier: &BezierCurve, view: &Viewport) {
    let points: Vec<DVec2> = linspace_f64(0.0, 1.0, 40)
        .iter()
        .map(|t| view.world_to_screen(bezier.eval(*t)))
        .collect();

    let handles = bezier
        .points
        .iter()
        .map(|p| view.world_to_screen(*p))
        .collect();

    cmd.linestring(handles).thickness(3.0).color(Color::PURPLE);
    cmd.linestring(points).thickness(5.0);
}

fn draw_track(
    cmd: &mut RenderCommands,
    track: &TrackSegment,
    world: &World,
    view: &Viewport,
) -> Option<()> {
    let points: Option<Vec<_>> = track
        .nodes
        .iter()
        .map(|id| world.nodes.get(*id).map(|n| n.pos))
        .collect();
    let points = points?;
    let bezier = BezierCurve::new(points);
    draw_bezier(cmd, &bezier, view);

    Some(())
}

fn draw_tracks(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for segment in world.segments.values() {
        _ = draw_track(cmd, segment, world, view);
    }

    for node in world.nodes.values() {
        let p = view.world_to_screen(node.pos);
        if node.tracks.is_empty() {
            cmd.circle(p).radii(8.0, 12.0).color(Color::GRAY);
        } else {
            cmd.circle(p).radius(9.0).color(Color::BLUE);
        };
    }
}

pub fn draw_world(
    cmd: &mut RenderCommands,
    events: &mut EventBus,
    input: &InputState,
    world: &World,
    screen_width: DVec2,
    mouse: DVec2,
    anim: &AnimationStates,
) {
    let view = Viewport::new(world.camera, screen_width);

    draw_grid_lines(cmd, &view);

    draw_font_ui(cmd, events, mouse, input);

    draw_tracks(cmd, world, &view);

    cmd.current_font_id = world.current_font_id;

    {
        let pw = DVec2::splat(0.0);

        let chars = "abcdefghijklmnopqrstuvwxyz";

        let n = (world.ticks / 100) as usize % chars.len();
        let c = chars.chars().nth(n).unwrap();

        let mut lines = vec![
            chars.to_uppercase().to_string(),
            chars.to_string(),
            format!("{} ticks", world.ticks),
            c.to_string(),
            format!("hovered_node   {:?}", world.hovered_node),
            format!("selected_nodes {:?}", world.selected_nodes),
            format!("pressed_node   {:?}", world.pressed_node),
        ];

        for (id, num, tween, state) in anim.animations() {
            lines.push(format!("{} {} {:?} {:0.2}", id, num, tween, state));
        }

        let text = lines.join("\n");

        for (off, color) in [(0.2, Color::BLACK.alpha(0.3)), (0.0, Color::WHITE)] {
            let off = DVec2::splat(off);
            let iso = view.w2s_iso((pw - off).into());
            let (text_cmd, _) = cmd.text(iso, &text, view.meters(2.0), color);
            cmd.apply(text_cmd);
        }

        {
            let p = DVec2::new(20.0, 20.0);
            let (text_cmd, _) = cmd.text(p, &text, 32.0, Color::WHITE);
            cmd.apply(text_cmd);
        }
    }

    cmd.circle(mouse).diameter(11.0).color(Color::RED);

    for id in &world.selected_nodes {
        if let Some(node) = world.nodes.get(*id) {
            cmd.circle(view.world_to_screen(node.pos))
                .radii(15.0, 25.0)
                .color(Color::ORANGE);
        }
    }
    if let Some(node_id) = world.hovered_node {
        if let Some(node) = world.nodes.get(node_id) {
            let p = view.world_to_screen(node.pos);
            cmd.circle(p).radii(25.0, 35.0).color(Color::PURPLE);
            let (b, _) = cmd.text(p, format!("{:?}", node_id), 40.0, Color::WHITE);
            cmd.apply(b);
        }
    }
}
