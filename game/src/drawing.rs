use crate::starling::prelude::*;
use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;

use crate::camera_controller::*;
use crate::canvas::Canvas;
use crate::game::GameState;
use crate::graph::*;
use crate::input::*;
use crate::scenes::*;
use crate::z_index::*;

pub fn draw_cross(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let dx = Vec2::new(size, 0.0);
    let dy = Vec2::new(0.0, size);
    gizmos.line_2d(p - dx, p + dx, color);
    gizmos.line_2d(p - dy, p + dy, color);
}

pub fn draw_x(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let s = size / 2.0;
    gizmos.line_2d(p + Vec2::new(-s, -s), p + Vec2::new(s, s), color);
    gizmos.line_2d(p + Vec2::new(s, -s), p + Vec2::new(-s, s), color);
}

pub fn draw_square(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    gizmos.rect_2d(
        Isometry2d::from_translation(p),
        Vec2::new(size, size),
        color,
    );
}

pub fn draw_diamond(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let s = size / 2.0;
    let pts = [0.0, PI / 2.0, PI, -PI / 2.0, 0.0].map(|a| p + rotate(Vec2::X * s, a));
    gizmos.linestrip_2d(pts, color);
}

pub fn draw_triangle(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let s = size;
    let pts =
        [0.0, 1.0 / 3.0, 2.0 / 3.0, 0.0].map(|a| p + rotate(Vec2::X * s, a * 2.0 * PI + PI / 2.0));
    gizmos.linestrip_2d(pts, color);
}

pub fn draw_circle(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    gizmos
        .circle_2d(Isometry2d::from_translation(p), size, color)
        .resolution(200);
}

pub fn draw_aabb(canvas: &mut Canvas, aabb: AABB, color: Srgba) {
    canvas
        .gizmos
        .rect_2d(Isometry2d::from_translation(aabb.center), aabb.span, color);
}

pub fn fill_aabb(canvas: &mut Canvas, aabb: AABB, color: Srgba) {
    // TODO get rid of this
    for t in linspace(0.0, 1.0, 10) {
        let s = aabb.from_normalized(Vec2::new(t, 0.0));
        let n = aabb.from_normalized(Vec2::new(t, 1.0));
        let w = aabb.from_normalized(Vec2::new(0.0, t));
        let e = aabb.from_normalized(Vec2::new(1.0, t));

        canvas.gizmos.line_2d(w, e, color);
        canvas.gizmos.line_2d(s, n, color);
    }
}

pub fn draw_and_fill_aabb(canvas: &mut Canvas, aabb: AABB, color: Srgba) {
    fill_aabb(canvas, aabb, color);
    draw_aabb(canvas, aabb, color);
}

pub fn draw_obb(canvas: &mut Canvas, obb: &OBB, color: Srgba, fill: bool) {
    // draw_cross(gizmos, obb.0.center, 30.0, color);
    // let mut corners = obb.corners().to_vec();
    // corners.push(*corners.get(0).expect("Expected a corner"));
    // gizmos.linestrip_2d(corners, color);
    let z = ZOrdering::Ui.as_f32();
    canvas.painter.reset();
    canvas.painter.set_color(color);
    if fill {
        canvas.painter.hollow = false;
        canvas.painter.thickness = 0.0;
    } else {
        canvas.painter.hollow = true;
        canvas.painter.thickness = 2.0;
    }
    canvas.painter.set_translation(obb.0.center.extend(z));
    canvas.painter.set_rotation(Quat::from_rotation_z(obb.1));
    canvas.painter.rect(obb.0.span);
}

pub fn draw_orbit(
    canvas: &mut Canvas,
    orb: &SparseOrbit,
    origin: DVec2,
    color: Srgba,
    draw_nodes: bool,
    ctx: &impl CameraProjection,
) {
    let peri: DVec2 = orb.periapsis();
    let apo = orb.apoapsis();

    let draw_node = |canvas: &mut Canvas, pos: DVec2, text: &'static str| {
        if !draw_nodes {
            return;
        }
        let p = pos + origin;
        let p = ctx.w2c(p);
        let z = ZOrdering::Orbit;
        let p = p.extend(z.as_f32());
        canvas.circle(p, 5.0, color);
        let tp = p.xy() + pos.as_vec2().normalize_or_zero() * 20.0;
        let d = pos.length();
        let text = format!("{} {}", text, distance_str(d));
        let anchor = if pos.x > 0.0 {
            Anchor::CenterLeft
        } else {
            Anchor::CenterRight
        };
        canvas
            .text(text, tp, 0.6)
            .set_z_order(ZOrdering::OrbitLabels)
            .set_anchor(anchor)
            .set_color(GRAY);
    };

    draw_node(canvas, peri, "PERI");

    if orb.ecc() >= 1.0 {
        // orb.will_escape() {
        let ta = if orb.is_hyperbolic() {
            let hrta = hyperbolic_range_ta(orb.ecc() as f32);
            linspace(-0.999 * hrta, 0.999 * hrta, 1000)
        } else {
            linspace(-PI, PI, 1000)
        };

        let points: Vec<_> = ta
            .iter()
            .filter_map(|t| {
                let p = orb.position_at(*t as f64);
                if p.length() > orb.body.soi as f64 {
                    return None;
                }
                Some(ctx.w2c(origin + p))
            })
            .collect();
        canvas.gizmos.linestrip_2d(points, color);
    } else {
        let b = orb.semi_minor_axis();
        let center = origin + (orb.periapsis() + orb.apoapsis()) / 2.0;
        let center = ctx.w2c(center);
        let rot = Quat::from_rotation_z(orb.arg_periapsis as f32);
        canvas.painter.reset();
        canvas
            .painter
            .set_translation(center.extend(ZOrdering::Orbit.as_f32()));
        canvas.painter.set_rotation(rot);
        canvas
            .painter
            .set_scale(Vec3::new(1.0, (b / orb.semi_major_axis) as f32, 1.0));
        canvas.painter.hollow = true;
        canvas.painter.thickness = 2.0;
        canvas.painter.set_color(color);
        canvas
            .painter
            .circle((orb.semi_major_axis * ctx.scale()) as f32 + 1.0);

        draw_node(canvas, apo, "APO");
    }
}

#[allow(unused)]
fn draw_orbit_between(
    gizmos: &mut Gizmos,
    orb: &SparseOrbit,
    origin: Vec2,
    color: Srgba,
    start: Nanotime,
    end: Nanotime,
    ctx: &impl CameraProjection,
) -> Option<()> {
    let points: Vec<_> = orb
        .sample_pos(start, end, 100.0, origin)?
        .into_iter()
        .map(|p| ctx.w2c(p))
        .collect();
    gizmos.linestrip_2d(points, color);
    Some(())
}

pub fn to_srgba(fl: [f32; 4]) -> Srgba {
    Srgba::new(fl[0], fl[1], fl[2], fl[3])
}

pub fn draw_thruster(
    gizmos: &mut Gizmos,
    thruster: &ThrusterModel,
    part_dims: Vec2,
    center: Vec2,
    scale: f32,
    angle: f32,
) {
    // along-plume direction
    let u = rotate(-Vec2::X, angle);

    // cross-plume direction
    let v = rotate(u, PI / 2.0);

    // corners of the business end of the thruster
    let p2 = center + (u * part_dims.x / 2.0 + v * part_dims.y / 2.0) * scale;
    let p3 = center + (u * part_dims.x / 2.0 - v * part_dims.y / 2.0) * scale;

    let c1 = to_srgba(thruster.primary_color);
    let c2 = to_srgba(thruster.secondary_color);

    let ul = rotate(u, thruster.plume_angle);
    let ur = rotate(u, -thruster.plume_angle);

    for s in linspace(0.0, 1.0, 13) {
        let length = thruster.plume_length * rand(0.6, 1.0) * ((s - 0.5) * PI).abs().cos();

        let p4 = p2 + ul * length * scale;
        let p5 = p3 + ur * length * scale;

        let color = c1.mix(&c2, rand(0.0, 1.0));
        let u = p2.lerp(p3, s);
        let v = p4.lerp(p5, s);
        gizmos.line_2d(u, v, color);
    }
}

#[allow(unused)]
fn draw_prograde_marker(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let mut draw_notch = |a: f32| {
        let start = p + rotate(Vec2::X * 0.5 * size, a);
        let end = p + rotate(Vec2::X * size, a);
        gizmos.line_2d(start, end, color);
    };

    draw_notch(0.0);
    draw_notch(PI / 2.0);
    draw_notch(PI);

    draw_circle(gizmos, p, size * 0.5, color);
}

pub fn make_separation_graph(
    src: &SparseOrbit,
    dst: &SparseOrbit,
    now: Nanotime,
) -> (Graph, Graph, Vec<DVec2>) {
    // t is in hours!
    let mut g = Graph::linspace(0.0, 48.0, 100);
    let mut v = Graph::linspace(0.0, 48.0, 100);

    let duration = src
        .period()
        .zip(dst.period())
        .map(|(s, d)| s.min(d) * 3)
        .unwrap_or(Nanotime::hours(16));

    let teval = tspace(now, now + duration, 300);

    let pv = apply(&teval, |t| {
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        p.pos - q.pos
    });

    let sep = |hours| {
        let t = now + Nanotime::secs_f64(hours * 3600.0);
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        p.pos.distance(q.pos)
    };

    let rvelx = |hours| {
        let t = now + Nanotime::secs_f64(hours * 3600.0);
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        p.vel.x - q.vel.x
    };

    let rvely = |hours| {
        let t = now + Nanotime::secs_f64(hours * 3600.0);
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        p.vel.y - q.vel.y
    };

    g.add_func(sep, WHITE);
    g.add_point(0.0, 0.0, true);
    g.add_point(0.0, 50.0, true);

    v.add_func(rvelx, ORANGE);
    v.add_func(rvely, TEAL);
    v.add_point(0.0, 0.0, true);

    (g, v, pv)
}

pub fn draw_pointing_vector(canvas: &mut Canvas, center: Vec2, r: f32, u: Vec2, color: Srgba) {
    let triangle_width = 22.0;
    let v = rotate(u, PI / 2.0);
    let p1 = center + u * r;
    let p2 = p1 + (v - u) * triangle_width;
    let p3 = p2 - v * triangle_width * 2.0;
    canvas.painter.reset();
    canvas.painter.set_color(color);
    canvas.painter.hollow = false;
    canvas
        .painter
        .set_translation(Vec3::Z * ZOrdering::Ui.as_f32());
    canvas.painter.triangle(p1, p2, p3);
}

pub fn draw_arc(
    painter: &mut ShapePainter,
    pos: Vec2,
    z: f32,
    color: Srgba,
    r: f32,
    start: f32,
    end: f32,
    thickness: f32,
) {
    painter.reset();
    painter.set_translation(pos.extend(z));
    painter.set_color(color);
    painter.hollow = true;
    painter.thickness = thickness;
    painter.cap = Cap::Round;
    painter.arc(r + thickness / 2.0, start, end);
}

pub fn is_blinking(wall_time: Nanotime) -> bool {
    let clock = (wall_time % Nanotime::secs(1)).to_secs();
    clock >= 0.5
}

// fn draw_event_animation(
//     gizmos: &mut Gizmos,
//     state: &GameState,
//     id: EntityId,
//     ctx: &impl CameraProjection,
// ) -> Option<()> {
//     let obj = state.universe.orbital_vehicles.get(&id)?.orbiter();
//     let p = obj.props().last()?;
//     let dt = Nanotime::hours(1);
//     let mut t = state.universe.stamp() + dt;
//     while t < p
//         .end()
//         .unwrap_or(state.universe.stamp() + Nanotime::days(5))
//     {
//         let pv = obj.pv(t, &state.universe.planets)?;
//         draw_circle(gizmos, ctx.w2c(pv.pos), 3.0, WHITE.with_alpha(0.2));
//         t += dt;
//     }
//     for prop in obj.props() {
//         if let Some((t, e)) = prop.stamped_event() {
//             let pv = obj.pv(t, &state.universe.planets)?;
//             draw_event_marker_at(gizmos, state.wall_time, &e, ctx.w2c(pv.pos));
//         }
//     }
//     if let Some(t) = p.end() {
//         let pv = obj.pv(t, &state.universe.planets)?;
//         draw_square(gizmos, ctx.w2c(pv.pos), 13.0, RED.with_alpha(0.8));
//     }
//     Some(())
// }

// fn draw_maneuver_plan(
//     canvas: &mut Canvas,
//     stamp: Nanotime,
//     plan: &ManeuverPlan,
//     origin: DVec2,
//     wall_time: Nanotime,
//     ctx: &impl CameraProjection,
// ) -> Option<()> {
//     let anim_dur = Nanotime::secs(2);
//     let s = (wall_time % anim_dur).to_secs() / anim_dur.to_secs();

//     for s in [s - 1.0, s - 0.5, s, s + 0.5, s + 1.0] {
//         let t_anim = plan.start() + plan.duration() * s;
//         let t_end: Nanotime = t_anim + plan.duration() * 0.2;
//         let positions: Vec<_> = tspace(t_anim, t_end, 30)
//             .iter()
//             .filter_map(|t| (*t >= stamp).then(|| plan.pv(*t)).flatten())
//             .map(|p| ctx.w2c(p.pos + origin))
//             .collect();

//         canvas.gizmos.linestrip_2d(positions, YELLOW);
//     }

//     for segment in &plan.segments {
//         if segment.end > stamp {
//             let pv = plan.pv(segment.end)?;
//             let p = ctx.w2c(pv.pos + origin);
//             draw_circle(&mut canvas.gizmos, p, 20.0, WHITE);
//         }
//     }
//     draw_orbit(canvas, &plan.terminal, origin, PURPLE, ctx);
//     Some(())
// }

pub fn draw_graph(
    canvas: &mut Canvas,
    graph: &Graph,
    bounds: AABB,
    input: Option<&InputState>,
) -> Option<()> {
    let map = |p: DVec2| bounds.from_normalized(aabb_stopgap_cast(p));

    {
        // axes
        let origin = graph.origin();
        let d = origin.with_y(0.0);
        let u = origin.with_y(1.0);
        let l = origin.with_x(0.0);
        let r = origin.with_x(1.0);
        canvas.gizmos.line_2d(map(l), map(r), GRAY.with_alpha(0.2));
        canvas.gizmos.line_2d(map(d), map(u), GRAY.with_alpha(0.2));
    }

    if let Some(p) = input
        .map(|i| i.position(MouseButt::Hover, FrameId::Current))
        .flatten()
    {
        if bounds.contains(p) {
            canvas.text("Graph!".to_uppercase(), p, 0.7);
        }
    }

    for signal in graph.signals() {
        let p = signal.points().map(|p| map(p)).collect::<Vec<_>>();
        canvas.gizmos.linestrip_2d(p, signal.color());
    }

    for p in graph.points() {
        if !AABB::unit().contains(aabb_stopgap_cast(p)) {
            continue;
        }
        draw_x(&mut canvas.gizmos, map(p), 10.0, WHITE.with_alpha(0.6));
    }

    Some(())
}

pub fn draw_bezier(gizmos: &mut Gizmos, bezier: &Bezier, color: Srgba) {
    let points: Vec<_> = linspace(0.0, 1.0, 20)
        .into_iter()
        .map(|t| bezier.eval(t))
        .collect();
    gizmos.linestrip_2d(points, color);
}

pub fn draw_game_state(gizmos: Gizmos, mut state: ResMut<GameState>, painter: ShapePainter) {
    let mut canvas = Canvas::new(gizmos, painter);

    GameState::draw(&mut canvas, &state);

    state.text_labels = canvas.text_labels;
    state.sprites = canvas.sprites;
}

pub fn draw_camera_info(
    canvas: &mut Canvas,
    state: &GameState,
    ctx: &impl CameraProjection,
    window_span: Vec2,
) {
    let meters = window_span.as_dvec2() / ctx.scale();
    let lower_bound = ctx.offset() - meters / 2.0;
    let upper_bound = ctx.offset() + meters / 2.0;

    let xl = lower_bound.x.ceil() as i64;
    let xu = upper_bound.x.floor() as i64;

    let yl = lower_bound.y.ceil() as i64;
    let yu = upper_bound.y.floor() as i64;

    let dist = (xu - xl).max(yu - yl);

    let mut step: i64 = 1;

    for s in [
        1,
        5,
        10,
        25,
        50,
        100,
        250,
        500,
        1_000,
        2_000,
        5_000,
        10_000,
        25_000,
        50_000,
        100_000,
        200_000,
        500_000,
        1_000_000,
        5_000_000,
        10_000_000,
        50_000_000,
        100_000_000,
        250_000_000,
        500_000_000,
        1_000_000_000,
    ] {
        let n = dist / s;
        if n < 15 {
            step = s;
            break;
        }
    }

    let label = format!(
        "{} {} {:0.1}",
        distance_str(step as f64),
        distance_str(ctx.distance()),
        ctx.angle().to_degrees(),
    );

    canvas
        .text(label, -window_span / 2.0 + Vec2::splat(40.0), 0.9)
        .set_anchor(Anchor::CenterLeft);

    let xl = (xl / step) * step;
    let xu = (xu / step) * step;

    let yl = (yl / step) * step;
    let yu = (yu / step) * step;

    let step = step.try_into().unwrap();

    let origin = ctx.origin() - ctx.offset();

    canvas.painter.reset();
    canvas.painter.set_color(WHITE.with_alpha(0.003));
    canvas.painter.thickness = 1.0;

    let len = state.input.screen_bounds.span.length() as f64 / ctx.scale();

    for x in (xl..=xu).step_by(step) {
        let wp = lower_bound.with_x(x as f64);
        let p = ctx.w2c(origin + wp - DVec2::Y * len);
        let q = ctx.w2c(origin + wp + DVec2::Y * len);
        canvas.painter.line(
            p.extend(ZOrdering::ScaleIndicator.as_f32()),
            q.extend(ZOrdering::ScaleIndicator.as_f32()),
        );
    }

    for y in (yl..=yu).step_by(step) {
        let wp = upper_bound.with_y(y as f64);
        let p = ctx.w2c(origin + wp - DVec2::X * len);
        let q = ctx.w2c(origin + wp + DVec2::X * len);
        canvas.painter.line(
            p.extend(ZOrdering::ScaleIndicator.as_f32()),
            q.extend(ZOrdering::ScaleIndicator.as_f32()),
        );
    }

    canvas.painter.set_color(WHITE.with_alpha(0.03));

    for a in 0..4 {
        let a = 0.5 * PI * a as f32;
        let c = ctx.w2c(origin);
        let p = Vec2::from_angle(a) * 100.0;
        let q = p * 3.0;
        canvas.painter.line(
            (c + p).extend(ZOrdering::ScaleIndicator.as_f32()),
            (c + q).extend(ZOrdering::ScaleIndicator.as_f32()),
        );
    }
}
