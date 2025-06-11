use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::onclick::OnClick;
use crate::scenes::{CameraProjection, Render, TextLabel};
use bevy::color::{palettes::css::*, Luminance};
use bevy::prelude::*;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug)]
pub struct SurfaceContext {
    plants: Vec<Plant>,
    wind_offset: f32,
    vehicle: Option<Vehicle>,
}

fn generate_plants() -> Vec<Plant> {
    let mut ret = Vec::new();

    for _ in 0..5 {
        let root = Vec2::new(rand(-300.0, 300.0), rand(-150.0, -50.0));

        let mut segments = Vec::new();
        if rand(0.0, 1.0) < 0.2 {
            let n_segments = randint(3, 6);
            for _ in 0..n_segments {
                let angle = rand(-0.4, 0.4);
                let length = rand(6.0, 11.0);
                segments.push((angle, length));
            }
        } else {
            for _ in 0..2 {
                let angle = rand(-0.4, 0.4);
                let length = rand(3.0, 7.0);
                segments.push((angle, length));
            }
        }

        let p = Plant::new(root, segments);
        ret.push(p);
    }

    ret
}

impl SurfaceContext {
    pub fn add_vehicle(&mut self, vehicle: Vehicle) {
        self.vehicle = Some(vehicle);
    }

    pub fn step(state: &mut GameState, dt: f32) {
        state.sim_speed = 0;
        let ctx = &mut state.surface_context;
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

        if let Some(v) = &mut ctx.vehicle {
            v.step(state.sim_time, Vec2::X, 1.0, false, PhysicsMode::RealTime);
        }
    }
}

impl Default for SurfaceContext {
    fn default() -> Self {
        SurfaceContext {
            plants: generate_plants(),
            wind_offset: 0.0,
            vehicle: None,
        }
    }
}

impl CameraProjection for SurfaceContext {
    fn origin(&self) -> Vec2 {
        Vec2::new(0.0, 30.0)
    }

    fn scale(&self) -> f32 {
        4.0
    }
}

fn draw_plant(gizmos: &mut Gizmos, p: &Plant, ctx: &impl CameraProjection) {
    draw_circle(gizmos, ctx.w2c(p.root), 6.0, WHITE);
    for (p, q) in p.segments() {
        let p = ctx.w2c(p);
        let q = ctx.w2c(q);
        draw_circle(gizmos, p, 4.0, WHITE);
        gizmos.line_2d(p, q, ORANGE);
    }
}

impl Render for SurfaceContext {
    fn background_color(_state: &GameState) -> Srgba {
        TEAL.with_luminance(0.3)
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.surface_context;

        if let Some(v) = &ctx.vehicle {
            draw_vehicle(&mut canvas.gizmos, v, Vec2::ZERO, 40.0, v.angle());
        }

        let p = ctx.w2c(Vec2::new(-400.0, 0.0));
        let q = ctx.w2c(Vec2::new(400.0, 0.0));

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
