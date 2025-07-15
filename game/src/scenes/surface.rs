use crate::camera_controller::LinearCameraController;
use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::*;
use crate::onclick::OnClick;
use crate::scenes::{CameraProjection, Render};
use crate::thrust_particles::*;
use bevy::color::{palettes::css::*, Alpha, Srgba};
use bevy::prelude::{Gizmos, KeyCode};
use layout::layout::Tree;
use starling::prelude::*;
use std::collections::HashSet;

#[derive(Debug)]
pub struct SurfaceContext {
    camera: LinearCameraController,
    selected: HashSet<usize>,
    particles: ThrustParticleEffects,
}

impl Default for SurfaceContext {
    fn default() -> Self {
        SurfaceContext {
            camera: LinearCameraController::new(Vec2::ZERO, 1.0, 2500.0),
            selected: HashSet::new(),
            particles: ThrustParticleEffects::new(),
        }
    }
}

pub fn to_srbga(fl: [f32; 4]) -> Srgba {
    Srgba::new(fl[0], fl[1], fl[2], fl[3])
}

impl SurfaceContext {
    pub fn camera(&self) -> &LinearCameraController {
        &self.camera
    }

    pub fn mouseover_vehicle(universe: &Universe, pos: Vec2) -> Option<(usize, &Vehicle)> {
        for (i, v) in universe.surface_vehicles.iter().enumerate() {
            let d = v.pv.pos_f32().distance(pos);
            let r = v.bounding_radius();
            if d < r {
                return Some((i, v));
            }
        }
        None
    }

    pub fn handle_input(&mut self, input: &InputState) {
        self.camera.handle_input(input);
    }

    pub fn on_game_tick(state: &mut GameState) {
        let ctx = &mut state.surface_context;

        ctx.camera.on_game_tick();

        // (|| -> Option<()> {
        //     let (pos, double) = if let Some(p) = state.input.double_click() {
        //         (p, true)
        //     } else {
        //         (state.input.on_frame(MouseButt::Left, FrameId::Down)?, false)
        //     };

        //     let add = state.input.is_pressed(KeyCode::ShiftLeft);
        //     if !add {
        //         ctx.selected.clear();
        //     }

        //     let pos = ctx.c2w(pos);
        //     let (idx, _) = Self::mouseover_vehicle(&state.universe, pos)?;
        //     ctx.selected.insert(idx);
        //     if double {
        //         // TODO fix this
        //         // ctx.follow_vehicle = true;
        //     }
        //     None
        // })();

        // (|| -> Option<()> {
        //     let rc = state.input.position(MouseButt::Right, FrameId::Current)?;
        //     let p = ctx.c2w(rc);

        //     let sep = 15.0;
        //     let spread = sep * ctx.selected.len() as f32 - sep;
        //     let center = Vec2::new(spread / 2.0, 0.0);

        //     for (i, idx) in ctx.selected.iter().enumerate() {
        //         let v = state.universe.surface_vehicles.get_mut(*idx);
        //         if let Some(v) = v {
        //             v.policy =
        //                 VehicleControlPolicy::PositionHold(p + Vec2::X * 15.0 * i as f32 - center);
        //         }
        //     }
        //     None
        // })();

        ctx.particles.step();

        for v in state.universe.surface_vehicles.iter_mut() {
            for t in v.thrusters_ref() {
                if !t.variant.is_thrusting() || t.variant.model().is_rcs {
                    continue;
                }

                ctx.particles.add(v, &t);
            }

            if v.pv.pos.y < 0.0 {
                v.pv.pos.y = 0.0;
                v.pv.vel.y = 0.0;
            }

            if v.pv.pos.y == 0.0 {
                v.pv.vel.x *= 0.98;
            }
        }
    }
}

impl CameraProjection for SurfaceContext {
    fn origin(&self) -> Vec2 {
        self.camera.origin()
    }

    fn scale(&self) -> f32 {
        self.camera.scale()
    }
}

fn draw_kinematic_arc(gizmos: &mut Gizmos, mut pv: PV, ctx: &impl CameraProjection, accel: Vec2) {
    let dt = 0.25;
    for _ in 0..100 {
        if pv.pos.y < 0.0 {
            return;
        }
        let q = ctx.w2c(pv.pos_f32());
        draw_circle(gizmos, q, 2.0, GRAY);
        pv.pos += pv.vel * dt;
        pv.vel += accel.as_dvec2() * dt;
    }
}

fn surface_scene_ui(state: &GameState) -> Tree<OnClick> {
    use crate::ui::*;
    use layout::layout::*;

    let vb = state.input.screen_bounds;
    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return Tree::new();
    }

    let top_bar = top_bar(state);

    let show_gravity = Node::text(
        Size::Grow,
        BUTTON_HEIGHT,
        format!("{}", state.universe.surface.gravity_vector()),
    );

    let increase_gravity = Node::button(
        "More Gravity",
        OnClick::IncreaseGravity,
        Size::Grow,
        BUTTON_HEIGHT,
    );

    let decrease_gravity = Node::button(
        "Less Gravity",
        OnClick::DecreaseGravity,
        Size::Grow,
        BUTTON_HEIGHT,
    );

    let main_area = Node::grow().invisible();

    let wrapper = Node::structural(350, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(show_gravity)
        .with_child(increase_gravity)
        .with_child(decrease_gravity);

    let layout = Node::new(vb.span.x, vb.span.y)
        .tight()
        .invisible()
        .down()
        .with_child(top_bar)
        .with_child(main_area.with_child(wrapper));

    Tree::new().with_layout(layout, Vec2::ZERO)
}

impl Render for SurfaceContext {
    fn background_color(state: &GameState) -> Srgba {
        let c = state.universe.surface.atmo_color;
        to_srbga([c[0], c[1], c[2], 1.0])
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.surface_context;

        {
            let bl = ctx.w2c(Vec2::new(-1000.0, -500.0));
            let tr = ctx.w2c(Vec2::new(1000.0, 0.0));
            let center = (tr + bl) / 2.0;
            let dims = tr - bl;

            if dims.x > 0.0 && dims.y > 0.0 {
                let color = state.universe.surface.land_color;
                let color = to_srbga([color[0], color[1], color[2], 1.0]);

                canvas
                    .sprite(center, 0.0, "error", -10.0, dims)
                    .set_color(color);
            }
        };

        ctx.particles.draw(canvas, ctx);

        for v in &state.universe.surface_vehicles {
            let pos = ctx.w2c(v.pv.pos_f32());
            draw_vehicle(canvas, v, pos, ctx.scale(), v.angle());
        }

        (|| -> Option<()> {
            let mouse_pos = ctx.c2w(state.input.current()?);
            let (_, vehicle) = Self::mouseover_vehicle(&state.universe, mouse_pos)?;
            let pos = ctx.w2c(vehicle.pv.pos_f32());
            draw_circle(
                &mut canvas.gizmos,
                pos,
                vehicle.bounding_radius() * ctx.scale() * 1.1,
                RED.with_alpha(0.3),
            );
            None
        })();

        for id in &ctx.selected {
            if let Some(v) = state.universe.surface_vehicles.get(*id) {
                let pos = ctx.w2c(v.pv.pos_f32());
                draw_circle(
                    &mut canvas.gizmos,
                    pos,
                    v.bounding_radius() * ctx.scale(),
                    ORANGE.with_alpha(0.3),
                );

                let sim = simulate_vehicle(v.clone(), state.universe.surface.gravity_vector());

                for (i, (pos, angle)) in sim.iter().enumerate() {
                    let p = ctx.w2c(*pos);
                    draw_x(&mut canvas.gizmos, p, 2.0, WHITE.with_alpha(0.3));
                    if i % 20 == 0 {
                        let q = ctx.w2c(pos + rotate(Vec2::X * 5.0, *angle));
                        canvas.gizmos.line_2d(p, q, YELLOW.with_alpha(0.7));
                    }
                }

                let target = if let VehicleControlPolicy::PositionHold(target) = v.policy {
                    target
                } else {
                    continue;
                };

                let p = ctx.w2c(v.pv.pos_f32());
                let q = ctx.w2c(target);
                draw_x(&mut canvas.gizmos, q, 2.0 * ctx.scale(), RED);

                let info = crate::scenes::craft_editor::vehicle_info(v);
                canvas.text(info, p, 0.01 * ctx.scale()).anchor_left();

                canvas.gizmos.line_2d(p, q, BLUE);
                if state.universe.surface.gravity_vector().length() > 0.0 {
                    draw_kinematic_arc(
                        &mut canvas.gizmos,
                        v.pv,
                        ctx,
                        state.universe.surface.gravity_vector(),
                    );
                }
            }
        }

        let p = ctx.w2c(Vec2::new(-400.0, 0.0));
        let q = ctx.w2c(Vec2::new(400.0, 0.0));

        // grid of 10 meter increments
        for i in (-100..100).step_by(10) {
            for j in (-100..100).step_by(10) {
                let p = Vec2::new(i as f32, j as f32);
                let p = ctx.w2c(p);
                draw_cross(&mut canvas.gizmos, p, 3.0, WHITE.with_alpha(0.1));
            }
        }

        canvas.gizmos.line_2d(p, q, WHITE);

        canvas.label(crate::scenes::orbital::date_label(state));

        Some(())
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        Some(surface_scene_ui(state))
    }
}
