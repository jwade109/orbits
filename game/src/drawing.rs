use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::ORANGE;
use bevy::prelude::*;
use starling::core::*;
use starling::orbit::*;
use starling::planning::*;

use std::time::Duration;

use crate::planetary::GameState;

pub fn alpha(color: Srgba, a: f32) -> Srgba {
    Srgba { alpha: a, ..color }
}

pub fn draw_x(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let dx = Vec2::new(size, 0.0);
    let dy = Vec2::new(0.0, size);
    gizmos.line_2d(p - dx, p + dx, color);
    gizmos.line_2d(p - dy, p + dy, color);
}

pub fn draw_square(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    gizmos.rect_2d(
        Isometry2d::from_translation(p),
        Vec2::new(size, size),
        color,
    );
}

pub fn draw_circle(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    gizmos
        .circle_2d(Isometry2d::from_translation(p), size, color)
        .resolution(200);
}

pub fn draw_orbit(origin: Vec2, orb: &Orbit, gizmos: &mut Gizmos, a: f32, base_color: Srgba) {
    if orb.eccentricity >= 1.0 {
        let n_points = 60;
        let range = 0.98 * hyperbolic_range_ta(orb.eccentricity);
        let points: Vec<_> = (0..n_points)
            .map(|i| {
                let t = (i as f32 / (n_points - 1) as f32) * 2.0 - 1.0;
                origin + orb.position_at(t * range)
            })
            .collect();
        gizmos.linestrip_2d(points, alpha(base_color, a))
    }

    // {
    //     let root = orb.pos() + origin;
    //     let t1 = root + orb.normal() * 60.0;
    //     let t2 = root + orb.tangent() * 60.0;
    //     let t3 = root + orb.vel() * 3.0;
    //     gizmos.line_2d(root, t1, GREEN);
    //     gizmos.line_2d(root, t2, GREEN);
    //     gizmos.line_2d(root, t3, PURPLE);
    // }

    let b = orb.semi_major_axis * (1.0 - orb.eccentricity.powi(2)).sqrt();
    let center: Vec2 = origin + (orb.periapsis() + orb.apoapsis()) / 2.0;
    let iso = Isometry2d::new(center, orb.arg_periapsis.into());
    gizmos
        .ellipse_2d(iso, Vec2::new(orb.semi_major_axis, b), alpha(base_color, a))
        .resolution(orb.semi_major_axis.clamp(40.0, 300.0) as u32);

    gizmos.circle_2d(
        Isometry2d::from_translation(origin + orb.periapsis()),
        4.0,
        alpha(RED, a),
    );

    if orb.eccentricity < 1.0 {
        gizmos.circle_2d(
            Isometry2d::from_translation(origin + orb.apoapsis()),
            4.0,
            alpha(WHITE, a),
        );
    }
}

pub fn draw_globe(gizmos: &mut Gizmos, p: Vec2, radius: f32, color: Srgba) {
    draw_circle(gizmos, p, radius, color);
    let c = alpha(color, 0.15);
    let iso = Isometry2d::from_translation(p);
    for s in [0.9, 0.6, 0.3] {
        gizmos.ellipse_2d(iso, Vec2::new(radius, radius * s), c);
        gizmos.ellipse_2d(iso, Vec2::new(radius * s, radius), c);
    }
    for (p1, p2) in [
        ((-radius, 0.0), (radius, 0.0)),
        ((0.0, -radius), (0.0, radius)),
    ] {
        gizmos.line_2d(p + Vec2::from(p1), p + Vec2::from(p2), c);
    }
}

pub fn draw_orbital_system(
    gizmos: &mut Gizmos,
    sys: &OrbitalSystem,
    stamp: Duration,
    origin: Vec2,
    scale: f32,
) {
    draw_shadows(gizmos, origin, sys.primary.radius, stamp);
    draw_globe(gizmos, origin, sys.primary.radius, WHITE);
    for (a, ds) in [(1.0, 1.0), (0.3, 0.98), (0.1, 0.95)] {
        draw_circle(gizmos, origin, sys.primary.soi * ds, alpha(ORANGE, a));
    }

    {
        let (b, _) = sys.barycenter();
        gizmos.circle_2d(Isometry2d::from_translation(origin + b), 6.0, PURPLE);
        draw_x(gizmos, b, 8.0, PURPLE);
    }

    for (_, orbit) in &sys.objects {
        let pv = orbit.pv_at_time(stamp);
        let color: Srgba = WHITE;
        draw_square(gizmos, origin + pv.pos, (9.0 * scale).min(9.0), color);
        draw_orbit(origin, orbit, gizmos, 0.05, GRAY);
    }

    for (_, orbit, subsys) in sys.subsystems.iter() {
        draw_orbit(origin, orbit, gizmos, 0.1, WHITE);

        let mut o = *orbit;
        o.semi_major_axis -= subsys.primary.soi;
        draw_orbit(origin, &o, gizmos, 0.3, RED);
        o.semi_major_axis += 2.0 * subsys.primary.soi;
        draw_orbit(origin, &o, gizmos, 0.3, RED);

        let pv = orbit.pv_at_time(stamp);
        draw_orbital_system(gizmos, subsys, stamp, origin + pv.pos, scale);
    }
}

pub fn draw_scalar_field(gizmos: &mut Gizmos, sys: &OrbitalSystem, origin: Vec2) {
    for y in (-1000..1000).step_by(10) {
        let pts: Vec<Vec2> = (-1000..1000)
            .step_by(10)
            .map(|x| {
                let p1 = Vec2::new(x as f32, y as f32);
                let z = sys.potential_at(p1, sys.epoch);
                origin + p1 + Vec2::Y * -(-z).sqrt()
            })
            .collect();
        gizmos.linestrip_2d(pts, alpha(WHITE, 0.1));
    }
}

pub fn draw_scalar_field_cell(
    gizmos: &mut Gizmos,
    sys: &OrbitalSystem,
    origin: Vec2,
    center: Vec2,
    step: f32,
    levels: &[i32],
) {
    let bl = center + Vec2::new(-step / 2.0, -step / 2.0);
    let br = center + Vec2::new(step / 2.0, -step / 2.0);
    let tl = center + Vec2::new(-step / 2.0, step / 2.0);
    let tr = center + Vec2::new(step / 2.0, step / 2.0);

    let pot: Vec<(Vec2, f32)> = [bl, br, tr, tl]
        .iter()
        .map(|p| (*p, sys.potential_at(*p, sys.epoch)))
        .collect();

    for level in levels {
        let mut pts = vec![];

        for i in 0..4 {
            let p1 = pot[i].0;
            let z1 = pot[i].1;
            let p2 = pot[(i + 1) % 4].0;
            let z2 = pot[(i + 1) % 4].1;

            let l = *level as f32;

            if z1 > l && z2 < l || z1 < l && z2 > l {
                let t = (l - z1) / (z2 - z1);
                let d = origin + p1.lerp(p2, t);
                pts.push(d);
            }
        }

        gizmos.linestrip_2d(pts, GREEN);
    }
}

pub fn draw_scalar_field_v2(
    gizmos: &mut Gizmos,
    sys: &OrbitalSystem,
    origin: Vec2,
    levels: &[i32],
) {
    let step = 200;
    for y in (-4000..=4000).step_by(step) {
        for x in (-4000..=4000).step_by(step) {
            let p = Vec2::new(x as f32, y as f32);
            draw_scalar_field_cell(gizmos, sys, origin, p, step as f32, levels);
            draw_square(gizmos, origin + p, step as f32, alpha(WHITE, 0.01));
        }
    }
}

pub fn draw_shadows(gizmos: &mut Gizmos, origin: Vec2, radius: f32, stamp: Duration) {
    let angle = stamp.as_secs_f32() / 1000.0;
    let u = rotate(Vec2::X, angle);
    let color = alpha(BLACK, 0.01);
    let steps = 20;
    for i in 0..steps {
        let y = (i as f32 / (steps - 1) as f32) * 2.0 - 1.0;
        let xoff = Vec2::X * radius * (1.0 - y.powi(2)).sqrt();
        let yoff = Vec2::Y * y * radius;
        let start = origin + rotate(xoff + yoff, angle);
        let end = start + u * 20000.0;
        gizmos.line_2d(start, end, color);
    }
}

pub fn draw_game_state(mut gizmos: Gizmos, state: Res<GameState>) {
    let stamp = state.system.epoch;

    draw_scalar_field_v2(&mut gizmos, &state.system, Vec2::ZERO, &state.draw_levels);

    draw_orbital_system(
        &mut gizmos,
        &state.system,
        stamp,
        Vec2::ZERO,
        state.target_scale,
    );

    gizmos.grid_2d(
        Isometry2d::from_translation(Vec2::ZERO),
        (100, 100).into(),
        (500.0, 500.0).into(),
        Srgba {
            alpha: 0.003,
            ..GRAY
        },
    );

    for (id, color, size) in [
        (state.primary_object, ORANGE, 80.0),
        (state.secondary_object, BLUE, 75.0),
    ] {
        if let Some(orbit) = state.system.lookup(id) {
            let pv = orbit.pv_at_time(stamp);
            draw_orbit(Vec2::ZERO, orbit, &mut gizmos, 1.0, color);
            draw_square(
                &mut gizmos,
                pv.pos,
                (size * state.target_scale).min(size),
                alpha(color, 0.7),
            );
        }
    }

    // {
    //     let start = state.system.epoch;
    //     let end = start + Duration::from_secs(100);
    //     let pos: Vec<_> =
    //         get_future_positions(&state.system, state.primary_object, start, end, 500)
    //             .iter()
    //             .map(|pvs| pvs.pv.pos)
    //             .collect();
    //     gizmos.linestrip_2d(pos, ORANGE);
    //     let pos: Vec<_> =
    //         get_future_positions(&state.system, state.secondary_object, start, end, 500)
    //             .iter()
    //             .map(|pvs| pvs.pv.pos)
    //             .collect();
    //     gizmos.linestrip_2d(pos, BLUE);

    //     let approach = get_approach_info(
    //         &state.system,
    //         state.primary_object,
    //         state.secondary_object,
    //         start,
    //         end,
    //         800.0,
    //     );
    //     for evt in approach.iter() {
    //         draw_circle(&mut gizmos, evt.0.pv.pos, 200.0, ORANGE);
    //         draw_x(&mut gizmos, evt.0.pv.pos, 30.0, ORANGE);
    //         draw_circle(&mut gizmos, evt.1.pv.pos, 200.0, BLUE);
    //         draw_x(&mut gizmos, evt.1.pv.pos, 30.0, BLUE);
    //         gizmos.line_2d(evt.0.pv.pos, evt.1.pv.pos, WHITE);
    //     }
    // }
}
