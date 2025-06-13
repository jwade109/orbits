use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::onclick::OnClick;
use crate::scenes::rpo::LinearCameraController;
use crate::scenes::{CameraProjection, Render, TextLabel};
use bevy::color::{palettes::css::*, Luminance};
use bevy::prelude::*;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug)]
pub struct SurfaceContext {
    // camera stuff
    camera: LinearCameraController,
    // real stuff
    plants: Vec<Plant>,
    wind_offset: f32,
    vehicle: Option<(PV, Vehicle)>,
    follow_vehicle: bool,
}

fn generate_plants() -> Vec<Plant> {
    let mut ret = Vec::new();

    for _ in 0..30 {
        let root = Vec2::new(rand(-100.0, 100.0), rand(-40.0, -10.0));

        let mut segments = Vec::new();
        if rand(0.0, 1.0) < 0.2 {
            let n_segments = randint(2, 4);
            for _ in 0..n_segments {
                let angle = rand(-0.4, 0.4);
                let length = rand(1.2, 2.3);
                segments.push((angle, length));
            }
        } else {
            for _ in 0..2 {
                let angle = rand(-0.4, 0.4);
                let length = rand(0.3, 0.9);
                segments.push((angle, length));
            }
        }

        let p = Plant::new(root, segments);
        ret.push(p);
    }

    ret
}

const ACCELERATION_DUE_TO_GRAVITY: Vec2 = Vec2::new(0.0, -3.2); // m/s^2;

impl SurfaceContext {
    pub fn add_vehicle(&mut self, vehicle: Vehicle) {
        self.vehicle = Some((PV::ZERO, vehicle));
    }

    pub fn step(state: &mut GameState, dt: f32) {
        state.sim_speed = 0;

        let ctx = &mut state.surface_context;

        if state.input.just_pressed(KeyCode::KeyF) {
            ctx.follow_vehicle = !ctx.follow_vehicle;
        }

        ctx.camera.update(dt, &state.input);

        if let Some((pv, v)) = &mut ctx.vehicle {
            let is_rcs = state.input.is_pressed(KeyCode::ControlLeft);
            let control = if state.input.is_pressed(KeyCode::ArrowUp) {
                Vec2::X
            } else if state.input.is_pressed(KeyCode::ArrowDown) {
                -Vec2::X
            } else if state.input.is_pressed(KeyCode::ArrowLeft) && is_rcs {
                Vec2::Y
            } else if state.input.is_pressed(KeyCode::ArrowRight) && is_rcs {
                -Vec2::Y
            } else {
                Vec2::ZERO
            };

            if state.input.is_pressed(KeyCode::ArrowLeft) && !is_rcs {
                v.turn(0.03);
            }
            if state.input.is_pressed(KeyCode::ArrowRight) && !is_rcs {
                v.turn(-0.03);
            }

            let dv = v.step(
                state.wall_time,
                control,
                0.2,
                is_rcs,
                PhysicsMode::RealTime,
                Nanotime::zero(),
            );

            pv.vel += dv.as_dvec2() + (ACCELERATION_DUE_TO_GRAVITY * dt).as_dvec2();
            pv.pos += pv.vel * dt as f64;

            if pv.pos.y < 0.0 {
                pv.pos.y = 0.0;
                pv.vel.y = 0.0;
            }
            if pv.pos.y == 0.0 {
                pv.vel.x *= 0.99;
            }

            if ctx.follow_vehicle {
                ctx.camera.center = pv.pos_f32();
            }
        }

        for _ in 0..12 {
            for p in &mut ctx.plants {
                p.step(dt, ctx.wind_offset);
            }
        }

        if state.input.is_pressed(KeyCode::KeyM) {
            ctx.wind_offset += 0.01;
        } else if state.input.is_pressed(KeyCode::KeyN) {
            ctx.wind_offset -= 0.01;
        }

        ctx.wind_offset = ctx.wind_offset.clamp(-0.4, 0.4);
    }
}

impl Default for SurfaceContext {
    fn default() -> Self {
        SurfaceContext {
            camera: LinearCameraController {
                center: Vec2::ZERO,
                target_center: Vec2::ZERO,
                scale: 1.0,
                target_scale: 1.0,
            },
            plants: generate_plants(),
            wind_offset: 0.0,
            vehicle: None,
            follow_vehicle: false,
        }
    }
}

impl CameraProjection for SurfaceContext {
    fn origin(&self) -> Vec2 {
        self.camera.center
    }

    fn scale(&self) -> f32 {
        self.camera.scale
    }
}

fn draw_plant(gizmos: &mut Gizmos, p: &Plant, ctx: &impl CameraProjection) {
    for (p, q) in p.segments() {
        let p = ctx.w2c(p);
        let q = ctx.w2c(q);
        gizmos.line_2d(p, q, ORANGE);
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

impl Render for SurfaceContext {
    fn background_color(_state: &GameState) -> Srgba {
        TEAL.with_luminance(0.3)
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.surface_context;

        if let Some((pv, v)) = &ctx.vehicle {
            draw_kinematic_arc(&mut canvas.gizmos, *pv, ctx, ACCELERATION_DUE_TO_GRAVITY);
            let pos = ctx.w2c(pv.pos_f32());
            draw_vehicle(&mut canvas.gizmos, v, pos, ctx.scale(), v.angle());
            let info = crate::scenes::craft_editor::vehicle_info(v);
            let info = format!("{}\n{}", pv, info);
            canvas.text(info, pos + Vec2::X * 400.0, 0.7).anchor_left();
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

        for p in &ctx.plants {
            draw_plant(&mut canvas.gizmos, p, ctx);
        }

        canvas.label(TextLabel::new(
            format!("{:0.2}", ctx.wind_offset),
            Vec2::splat(-300.0),
            1.0,
        ));

        Some(())
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        Some(crate::ui::basic_scenes_layout(state))
    }
}
