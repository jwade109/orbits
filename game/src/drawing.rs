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

pub fn draw_orbit(origin: Vec2, orb: &Orbit, gizmos: &mut Gizmos, alpha: f32, base_color: Srgba) {
    if orb.eccentricity >= 1.0 {
        let n_points = 30;
        let theta_inf = f32::acos(-1.0 / orb.eccentricity);
        let points: Vec<_> = (-n_points..n_points)
            .map(|i| 0.98 * theta_inf * i as f32 / n_points as f32)
            .map(|t| origin + orb.position_at(t))
            .collect();
        gizmos.linestrip_2d(points, Srgba { alpha: 0.05, ..RED })
    }

    let color = Srgba {
        alpha,
        ..base_color
    };

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
        .ellipse_2d(iso, Vec2::new(orb.semi_major_axis, b), color)
        .resolution(orb.semi_major_axis.clamp(3.0, 200.0) as u32);

    gizmos.circle_2d(
        Isometry2d::from_translation(origin + orb.periapsis()),
        4.0,
        Srgba { alpha, ..RED },
    );

    if orb.eccentricity < 1.0 {
        gizmos.circle_2d(
            Isometry2d::from_translation(origin + orb.apoapsis()),
            4.0,
            Srgba { alpha, ..WHITE },
        );
    }
}

pub fn draw_orbital_system(
    gizmos: &mut Gizmos,
    sys: &OrbitalSystem,
    stamp: Duration,
    origin: Vec2,
) {
    draw_scalar_field(gizmos, sys, origin);

    draw_circle(gizmos, origin, sys.primary.radius, WHITE);
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
        draw_square(gizmos, origin + pv.pos, 9.0, color);
        // draw_orbit(origin, orbit, gizmos, 0.05, GRAY);
    }

    for (_, orbit, subsys) in sys.subsystems.iter() {
        draw_orbit(origin, orbit, gizmos, 0.1, WHITE);
        let pv = orbit.pv_at_time(stamp);
        draw_orbital_system(gizmos, subsys, stamp, origin + pv.pos);
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
            }).collect();
        gizmos.linestrip_2d(pts, alpha(WHITE, 0.1));
    }
}

pub fn draw_game_state(mut gizmos: Gizmos, state: Res<GameState>) {
    let stamp = state.system.epoch;

    draw_orbital_system(&mut gizmos, &state.system, stamp, Vec2::ZERO);

    gizmos.grid_2d(
        Isometry2d::from_translation(Vec2::ZERO),
        (100, 100).into(),
        (500.0, 500.0).into(),
        Srgba {
            alpha: 0.003,
            ..GRAY
        },
    );

    if let Some(p) = state
        .system
        .transform_from_id(state.primary_object, state.system.epoch)
    {
        draw_square(
            &mut gizmos,
            p.pos,
            80.0,
            Srgba {
                alpha: 0.3,
                ..ORANGE
            },
        );
    }

    if let Some(p) = state
        .system
        .transform_from_id(state.secondary_object, state.system.epoch)
    {
        draw_square(&mut gizmos, p.pos, 75.0, Srgba { alpha: 0.3, ..BLUE });
    }

    {
        let start = state.system.epoch;
        let end = start + Duration::from_secs(100);
        let pos: Vec<_> =
            get_future_positions(&state.system, state.primary_object, start, end, 500)
                .iter()
                .map(|pvs| pvs.pv.pos)
                .collect();
        gizmos.linestrip_2d(pos, ORANGE);
        let pos: Vec<_> =
            get_future_positions(&state.system, state.secondary_object, start, end, 500)
                .iter()
                .map(|pvs| pvs.pv.pos)
                .collect();
        gizmos.linestrip_2d(pos, BLUE);

        let approach = get_approach_info(
            &state.system,
            state.primary_object,
            state.secondary_object,
            start,
            end,
            800.0,
        );
        for evt in approach.iter() {
            draw_circle(&mut gizmos, evt.0.pv.pos, 200.0, ORANGE);
            draw_x(&mut gizmos, evt.0.pv.pos, 30.0, ORANGE);
            draw_circle(&mut gizmos, evt.1.pv.pos, 200.0, BLUE);
            draw_x(&mut gizmos, evt.1.pv.pos, 30.0, BLUE);
            gizmos.line_2d(evt.0.pv.pos, evt.1.pv.pos, WHITE);
        }
    }

    // for object in state.system.objects.iter() {
    //     if let Some((body, pv)) = object.body.zip(object.prop.pv_at(stamp)) {
    //         let iso = Isometry2d::from_translation(pv.pos);
    //         gizmos.circle_2d(iso, body.radius, WHITE);
    //         gizmos
    //             .circle_2d(
    //                 iso,
    //                 body.soi,
    //                 Srgba {
    //                     alpha: 0.3,
    //                     ..ORANGE
    //                 },
    //             )
    //             .resolution(200);

    //         // shadows for this body
    //         let angle = state.sim_time.as_secs_f32() / 1000.0;
    //         let u = rotate(Vec2::X, angle);
    //         let color = Srgba {
    //             alpha: 0.004,
    //             ..GRAY
    //         };
    //         let steps = 50;
    //         for i in 0..steps {
    //             let y = (i as f32 / (steps - 1) as f32) * 2.0 - 1.0;
    //             let xoff = Vec2::X * body.radius * (1.0 - y.powi(2)).sqrt();
    //             let yoff = Vec2::Y * y * body.radius;
    //             let start = pv.pos + rotate(xoff + yoff, angle);
    //             let end = start + u * 10000.0;
    //             gizmos.line_2d(start, end, color);
    //         }
    //     }
    // }

    // let mut lattice = vec![]; // generate_square_lattice(Vec2::ZERO, 10000, 200);

    // for obj in state.system.objects.iter() {
    //     if let Some(body) = obj.body {
    //         if let Some(center) = state.system.global_transform(&obj.prop, stamp) {
    //             let minilat =
    //                 generate_circular_log_lattice(center.pos, body.radius + 5.0, body.soi * 2.0);
    //             lattice.extend(minilat);
    //         }
    //     }
    // }

    // let gravity = lattice.iter().map(|p| state.system.gravity_at(*p));
    // let potential = lattice.iter().map(|p| state.system.potential_at(*p));
    // let max_potential = state.system.potential_at((500.0, 500.0).into());

    // if state.show_gravity_field {
    //     for (grav, p) in gravity.zip(&lattice) {
    //         let a = grav.angle_to(Vec2::X);
    //         let r = 0.5 * a.cos() + 0.5;
    //         let g = 0.5 * a.sin() + 0.5;
    //         let color = Srgba {
    //             red: r,
    //             green: g,
    //             blue: 1.0,
    //             alpha: 0.8,
    //         };
    //         draw_square(&mut gizmos, *p, 10.0, color);
    //         gizmos.line_2d(*p, p + grav.normalize() * 15.0, color);
    //     }
    // }
    // if state.show_potential_field {
    //     for (pot, p) in potential.zip(&lattice) {
    //         let r = (((pot / max_potential * 5.0) as u32) as f32 / 5.0).sqrt();
    //         let color = Srgba {
    //             red: r,
    //             green: 0.0,
    //             blue: 1.0 - r,
    //             alpha: 0.7,
    //         };
    //         draw_square(&mut gizmos, *p, 20.0, color);
    //     }
    // }
}
