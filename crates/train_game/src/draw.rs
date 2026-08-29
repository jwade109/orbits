use crate::bezier::BezierCurve;
use crate::event_bus::EventBus;
use crate::railcar::RailCar;
use crate::terrain::{TERRAIN_CHUNK_WIDTH_METERS, TerrainChunk};
use crate::track::{Terminus, TrackSegment};
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
    let (tcmd, extent) = cmd.text(p + padding, text, 22.0, Color::WHITE);
    let full_extent = extent + padding * 2.0;
    let rect_origin = p - extent.y * DVec2::Y;
    let aabb = AABB::from_arbitrary(rect_origin.as_vec2(), (rect_origin + full_extent).as_vec2());
    let contains = aabb.contains(mouse.as_vec2());
    let alpha = contains as u8 as f64 * 0.2 + 0.9;
    cmd.rect(rect_origin)
        .dims(full_extent)
        .color(Color::BLUE.alpha(alpha));
    cmd.apply(tcmd);
    (
        full_extent,
        input.just_pressed(rdev::Button::Left) && contains,
    )
}

fn draw_terrain(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for chunk in world.chunks.values() {
        let iso = view.w2s_iso(chunk.isometry());
        let dims = DVec2::splat(view.meters(TERRAIN_CHUNK_WIDTH_METERS));
        cmd.rect(iso).dims(dims);
    }
}

fn draw_grid_lines(cmd: &mut RenderCommands, view: &Viewport) {
    let n_lines = 500;

    let spacing = if view.zoom() > 20.0 {
        5
    } else if view.zoom() > 2.0 {
        25
    } else if view.zoom() > 0.2 {
        250
    } else if view.zoom() > 0.02 {
        1000
    } else {
        10000
    };

    for x in -n_lines..=n_lines {
        let x = x * spacing;
        let s = DVec2::new(x as f64, -(spacing * n_lines) as f64);
        let e = DVec2::new(x as f64, (spacing * n_lines) as f64);
        cmd.line(view.world_to_screen(s), view.world_to_screen(e))
            .color(Color::GRAY.alpha(0.5))
            .thickness(3.0);
        let s = DVec2::new(-(spacing * n_lines) as f64, x as f64);
        let e = DVec2::new((spacing * n_lines) as f64, x as f64);
        cmd.line(view.world_to_screen(s), view.world_to_screen(e))
            .color(Color::GRAY.alpha(0.5))
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

fn draw_bezier(
    cmd: &mut RenderCommands,
    bezier: &BezierCurve,
    view: &Viewport,
    handles: bool,
    color: Color,
    thickness: f64,
) {
    if handles && !bezier.is_linear() {
        let handles = bezier.points().map(|p| view.world_to_screen(*p)).collect();
        cmd.linestring(handles).thickness(3.0).color(Color::PURPLE);
    }

    let points = bezier
        .linestring(0.0, 1.0, 30)
        .into_iter()
        .map(|p| view.world_to_screen(p.translation.as_dvec2()))
        .collect();

    cmd.linestring(points).thickness(thickness).color(color);
}

fn draw_track(
    cmd: &mut RenderCommands,
    track: &TrackSegment,
    world: &World,
    view: &Viewport,
    handles: bool,
    color: Color,
    thickness: f64,
) -> Option<()> {
    let points: Option<Vec<_>> = track
        .nodes
        .iter()
        .map(|id| world.nodes.get(*id).map(|n| n.pos()))
        .collect();
    let points = points?;
    let bezier = BezierCurve::new(points)?;
    draw_bezier(cmd, &bezier, view, handles, color, thickness);

    Some(())
}

fn draw_tracks(cmd: &mut RenderCommands, world: &World, view: &Viewport, handles: bool) {
    for segment in world.segments.values() {
        _ = draw_track(cmd, segment, world, view, handles, Color::BLACK, 6.0);
    }
}

fn draw_track_centers(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for segment in world.segments.values() {
        let p = view.world_to_screen(segment.center.translation.as_dvec2());
        cmd.circle(p).radii(7.0, 12.0).color(Color::GRAY);
    }
}

fn draw_track_nodes(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for node in world.nodes.values() {
        let p = view.world_to_screen(node.pos());
        if node.tracks.is_empty() {
            cmd.circle(p).radii(8.0, 12.0).color(Color::GRAY);
        } else if node.is_semantic() {
            cmd.circle(p).radius(9.0).color(Color::BLUE);
        } else {
            cmd.circle(p).radius(6.0).color(Color::RED);
        }
    }
}

fn draw_track_junctions(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for node in world.nodes.values() {
        if node.is_switch() {
            let iso = node.isometry();

            let p = iso.offset(Vec2::X * 2.0).translation.as_dvec2();
            let q = iso.offset(-Vec2::X * 2.0).translation.as_dvec2();

            let p = view.world_to_screen(p);
            let q = view.world_to_screen(q);

            cmd.line(p, q)
                .thickness(view.meters(2.0))
                .color(Color::GRAY);

            let p = iso.offset(Vec2::Y * 3.0).translation.as_dvec2();
            let q = iso.offset(-Vec2::Y * 3.0).translation.as_dvec2();

            let p = view.world_to_screen(p);
            let q = view.world_to_screen(q);

            cmd.line(p, q).thickness(12.0).color(Color::RED);

            cmd.isometry(view.w2s_iso(iso), view.meters(5.0));
        }
    }
}

pub fn draw_railcar(cmd: &mut RenderCommands, iso: impl Into<Isometry2d>, view: &Viewport) {
    let dims = DVec2::new(
        view.meters(RailCar::LENGTH_METERS).max(11.0),
        view.meters(RailCar::WIDTH_METERS).max(11.0),
    );
    let iso = iso.into();
    let iso = view.w2s_iso(iso);
    cmd.rect(iso)
        .color(Color::BLUE.alpha(0.8))
        .dims(dims)
        .centered();
    // cmd.circle(iso.translation.as_dvec2())
    //     .radius(7.0)
    //     .color(Color::ORANGE);
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
    cmd.current_font_id = world.current_font_id;

    draw_terrain(cmd, world, &view);
    if world.show_detail {
        draw_track_bounds(cmd, world, &view);
        draw_track_chunk_occupancy(cmd, world, &view);
    }
    draw_grid_lines(cmd, &view);
    if !world.show_detail {
        draw_track_junctions(cmd, world, &view);
    }
    draw_tracks(cmd, world, &view, world.show_detail);
    draw_railcars(cmd, world, &view);

    if world.show_detail {
        draw_track_nodes(cmd, world, &view);
        draw_track_centers(cmd, world, &view);
    }

    {
        let p = DVec2::new(20.0, screen_width.y - 30.0);

        let chars = "abcdefghijklmnopqrstuvwxyz";

        let n = (world.ticks / 100) as usize % chars.len();
        let c = chars.chars().nth(n).unwrap();

        let mut lines = vec![
            chars.to_uppercase().to_string(),
            chars.to_string(),
            format!("{} ticks", world.ticks),
            c.to_string(),
            format!("zoom           {:0.1}", world.camera.zoom),
            format!("hovered_node   {:?}", world.hovered_node),
            format!("selected_nodes {:?}", world.selected_nodes),
            format!("pressed_node   {:?}", world.pressed_node),
            format!("hovered_track  {:?}", world.hovered_track),
            format!("selected_track {:?}", world.selected_track),
        ];

        for (id, num, tween, state) in anim.animations() {
            lines.push(format!("{} {} {:?} {:0.2}", id, num, tween, state));
        }

        let text = lines.join("\n");

        for (off, color) in [(0.0, Color::WHITE)] {
            let p = p - DVec2::splat(off);
            let (text_cmd, extent) = cmd.text(p, &text, 28.0, color);

            cmd.rect(p - extent.y * DVec2::Y)
                .dims(extent)
                .color(Color::BLACK.alpha(0.4));
            cmd.apply(text_cmd);
        }
    }

    draw_hovered_track(cmd, world, &view);
    draw_selected_nodes(cmd, world, &view);
    draw_calculated_route(cmd, world, &view);
    draw_hovered_node(cmd, world, &view);
    draw_hovered_chunk(cmd, world, &view);
    draw_font_ui(cmd, events, mouse, input);

    cmd.circle(mouse).diameter(11.0).color(Color::RED);

    {
        let mouse_world = view.screen_to_world(mouse);
        let text = format!("{mouse_world:0.2}");

        let (b, _) = cmd.text((18.0, 48.0), &text, 32.0, Color::BLACK.alpha(0.7));
        cmd.apply(b);

        let (b, _) = cmd.text((20.0, 50.0), &text, 32.0, Color::WHITE);
        cmd.apply(b);
    }
}

fn draw_hovered_chunk(cmd: &mut RenderCommands, world: &World, view: &Viewport) -> Option<()> {
    let index = world.hovered_chunk?;
    let id = *world.chunk_map.get(&index)?;
    let chunk = world.chunks.get(id)?;

    let text = format!("{:?}\n{:?}\n{:?}", index, chunk.nodes(), chunk.tracks());

    let size = 32.0f64.max(view.meters(3.0));

    let iso = index.isometry();
    let (b, _) = cmd.text(view.w2s_iso(iso), text, size, Color::WHITE);
    cmd.apply(b);

    Some(())
}

fn draw_calculated_route(cmd: &mut RenderCommands, world: &World, view: &Viewport) -> Option<()> {
    let route = world.calculated_route.as_ref()?;

    for segment_id in route.segments() {
        let track = world.segments.get(*segment_id)?;
        draw_track(cmd, track, world, view, false, Color::PURPLE, 16.0);
    }

    Some(())
}

fn draw_track_bounds(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for track in world.segments.values() {
        let iso = Isometry2d::from_pos(track.lower);
        let mut dims = track.upper - track.lower;
        dims.x = view.meters(dims.x);
        dims.y = view.meters(dims.y);
        let iso = view.w2s_iso(iso);
        cmd.frame(iso, dims)
            .thickness(3.0)
            .color(Color::BLUE.alpha(0.2));
    }
}

fn draw_track_chunk_occupancy(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for track in world.segments.values() {
        for id in &track.chunks {
            let iso = view.w2s_iso(id.isometry());
            let dims = DVec2::splat(view.meters(TERRAIN_CHUNK_WIDTH_METERS));
            cmd.rect(iso).dims(dims).color(Color::BLUE.alpha(0.1));
        }
    }
}

fn draw_railcars(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for car in world.cars.values() {
        let Some(track) = world.segments.get(car.segment) else {
            continue;
        };

        let iso = track.eval_at(car.origin, car.pos);
        draw_railcar(cmd, iso, &view);
    }
}

fn draw_hovered_track(cmd: &mut RenderCommands, world: &World, view: &Viewport) -> Option<()> {
    let track_id = world.hovered_track?;
    let track = world.segments.get(track_id)?;

    let color = Color::ORANGE;
    let p = view.world_to_screen(track.center.translation.as_dvec2());

    let s = view.world_to_screen(track.eval_at(Terminus::Start, 0.0).translation.as_dvec2());

    draw_track(cmd, track, world, &view, false, color, 15.0);
    cmd.circle(s).radii(12.0, 20.0).color(Color::FOREST_GREEN);
    cmd.circle(p).radii(25.0, 35.0).color(Color::GRAY);
    let text = format!(
        "{}\n{:0.1} meters\n{} => {}",
        track_id,
        track.length,
        track.start_node(),
        track.end_node()
    );
    let (b, _) = cmd.text(p, text, 32.0, Color::WHITE);
    cmd.apply(b);

    Some(())
}

fn draw_selected_nodes(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for id in &world.selected_nodes {
        if let Some(node) = world.nodes.get(*id) {
            cmd.circle(view.world_to_screen(node.pos()))
                .radii(15.0, 25.0)
                .color(Color::ORANGE);
        }
    }
}

fn draw_hovered_node(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    if let Some(node_id) = world.hovered_node {
        if let Some(node) = world.nodes.get(node_id) {
            for track_id in node.linked_tracks() {
                let Some(track) = world.segments.get(*track_id) else {
                    continue;
                };
                draw_track(cmd, track, world, view, false, Color::BLUE.alpha(0.7), 12.0);
            }

            let p = view.world_to_screen(node.pos());
            cmd.circle(p).radii(25.0, 35.0).color(Color::PURPLE);
            let text = format!(
                "{:?}\n{:0.1}\n{:?}\n{:?}",
                node_id,
                node.pos(),
                node.forward_connections,
                node.backward_connections
            );
            let (b, _) = cmd.text(p, text, 32.0, Color::WHITE);
            cmd.apply(b);
        }
    }
}
